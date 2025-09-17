use crate::{
    entity::{
        esse_interest_channel, peer_channel, peer_people,
        prelude::{EsseInterestChannel, GlobalApiConfig, UserAccount, UserChat, UserScraper},
        user_chat, user_scraper,
    },
    scraper::Scraper,
    stdin_read_line,
    types::{ApiConfig, FrozenSession, PackedChat},
};
use anyhow::Result;
use anyhow::anyhow;
use chrono::{DateTime, Local};
use sea_orm::{
    ActiveValue::{NotSet, Set},
    Condition, Database, IntoActiveModel, QueryOrder,
    prelude::*,
};
use tokio::sync::oneshot;
use tracing::{debug, info, warn};

fn now() -> DateTimeWithTimeZone {
    let now_local: DateTime<Local> = Local::now();
    now_local.with_timezone(&now_local.offset())
}

pub async fn login_async(
    api_config: ApiConfig,
    phone: String,
    code: oneshot::Receiver<String>,
) -> Result<FrozenSession> {
    let scraper = Scraper::login_async(api_config, &phone, code).await?;
    Ok(scraper.freeze())
}

pub async fn unfreeze_and<R>(
    frozen: FrozenSession,
    api_config: ApiConfig,
    with: impl Fn(&Scraper) -> Result<R>,
) -> Result<R> {
    let scraper = Scraper::unfreeze(frozen, api_config).await?;
    with(&scraper)
}

pub async fn connect_db() -> Result<DatabaseConnection> {
    warn!("连接到数据库");
    dotenv::dotenv().unwrap();
    let url = dotenv::var("DATABASE_URL".to_owned())?;
    let db = Database::connect(url).await?;
    Ok(db)
}

pub async fn resume_scraper(db: &impl ConnectionTrait) -> Result<Option<(Uuid, Scraper)>> {
    info!("获取冻结会话");
    let ret = if let Some(scraper) = UserScraper::find()
        .filter(user_scraper::Column::InUse.eq(false))
        .one(db)
        .await?
    {
        debug!("{:?}", scraper.frozen_session);

        debug!("get relate api_config");
        let api_config = scraper
            .find_related(GlobalApiConfig)
            .one(db)
            .await?
            .ok_or(anyhow!("relate global_api_config not found"))?;
        debug!("{:?}", api_config);

        debug!("create scraper instance");
        let s = Scraper::unfreeze(scraper.frozen_session.clone(), api_config.into()).await?;

        debug!("set db scraper state");
        let mut scraper = scraper.into_active_model();
        scraper.in_use = Set(true);
        let id = scraper.update(db).await?.id;

        Some((id, s))
    } else {
        warn!("未找到暂停的会话");
        None
    };
    Ok(ret)
}

pub async fn create_scraper_from_stdin(db: &impl ConnectionTrait) -> Result<(Uuid, Scraper)> {
    info!("从stdin输入验证码登录以创建爬虫会话");

    debug!("get global_api_config");
    let api_config = GlobalApiConfig::find()
        .one(db)
        .await?
        .ok_or(anyhow!("global_api_config not found"))?;
    let config_id = api_config.id;

    debug!("get user_account");
    let account = UserAccount::find()
        .one(db)
        .await?
        .ok_or(anyhow!("user_account not found"))?;
    let phone = account.phone.clone();

    debug!("get confirm code from stdin");
    let code = stdin_read_line(format!("请输入TG号为{}的验证码", phone));

    debug!("create scraper");
    let scraper = Scraper::login_async(api_config.into(), &phone, code).await?;
    let frozen = scraper.freeze();

    debug!("insert scraper in db");
    let scraper_model = user_scraper::ActiveModel {
        id: NotSet,
        api_config: Set(config_id),
        account: Set(account.id),
        frozen_session: Set(frozen),
        in_use: Set(true),
        ..Default::default()
    };
    let id = scraper_model.insert(db).await?.id;

    Ok((id, scraper))
}

pub async fn sync_chat(
    db: &impl ConnectionTrait,
    id: Uuid,
    scraper: &Scraper,
) -> Result<Vec<PackedChat>> {
    info!("枚举当前聊天并发送到数据库");
    let mut ret = Vec::new();
    let mut user_chat_to_insert = Vec::new();
    let mut peer_people_to_insert = Vec::new();
    let mut peer_channel_to_insert = Vec::new();

    for (username, chat) in scraper.list_chats_with_username().await? {
        ret.push(chat.clone());
        user_chat_to_insert.push(user_chat::ActiveModel {
            user_scraper: Set(id),
            packed_chat: Set(chat),
            username: Set(username),
            ..Default::default()
        });
        if chat.0.is_channel() {
            peer_channel_to_insert.push(peer_channel::ActiveModel {
                channel_id: Set(chat.0.id),
                ..Default::default()
            });
        }
        if chat.0.is_user() {
            peer_people_to_insert.push(peer_people::ActiveModel {
                people_id: Set(chat.0.id),
                ..Default::default()
            });
        }
    }
    // 将数据插入user_chat表
    user_chat::Entity::insert_many(user_chat_to_insert)
        .exec(db)
        .await?;

    // 将数据同步插入peer_channel和peer_people表中
    {
        peer_people::Entity::insert_many(peer_people_to_insert)
            .exec(db)
            .await?;
        peer_channel::Entity::insert_many(peer_channel_to_insert)
            .exec(db)
            .await?;
    }

    Ok(ret)
}

pub async fn exit_scraper(db: &impl ConnectionTrait, id: Uuid, scraper: Scraper) -> Result<()> {
    info!("退出爬虫，更新数据库状态");

    debug!("freeze scraper instence");
    let frozen = scraper.freeze();
    let scraper_update = user_scraper::ActiveModel {
        id: Set(id),
        updated_at: Set(now()),
        api_config: NotSet,
        account: NotSet,
        frozen_session: Set(frozen),
        in_use: Set(false),
    };
    debug!("update scraper in db");
    scraper_update.update(db).await?;
    Ok(())
}

pub async fn resolve_username(
    db: &impl ConnectionTrait,
    id: Uuid,
    scraper: &Scraper,
    username: &str,
) -> Result<Option<PackedChat>> {
    // 查询user_chat作为缓存返回
    if let Some(chat) = UserChat::find()
        .filter(
            Condition::all()
                .add(user_chat::Column::Username.eq(username))
                .add(user_chat::Column::UserScraper.eq(id)),
        )
        .one(db)
        .await?
    {
        return Ok(Some(chat.packed_chat));
    }

    let chat = if let Some(chat) = scraper.resolve_username(&username).await? {
        chat
    } else {
        return Ok(None);
    };

    // 存入user_chat
    user_chat::ActiveModel {
        user_scraper: Set(id),
        username: Set(Some(username.to_string())),
        packed_chat: Set(chat),
        ..Default::default()
    }
    .insert(db)
    .await?;

    // 同步到peer系列库中
    if chat.0.is_user() {
        peer_people::ActiveModel {
            people_id: Set(chat.0.id),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }
    if chat.0.is_channel() {
        peer_channel::ActiveModel {
            channel_id: Set(chat.0.id),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }

    Ok(Some(chat))
}

pub async fn get_stale_esse_channel(
    db: &impl ConnectionTrait,
    id: Uuid,
    scraper: &Scraper,
) -> Result<PackedChat> {
    // 循环直到找到最老的esse对应的packed_chat
    loop {
        let stale_esse = EsseInterestChannel::find()
            .order_by_asc(esse_interest_channel::Column::UpdatedAt)
            .one(db)
            .await?
            .ok_or(anyhow!("esse channel not found"))?;
        if let Some(chat) = resolve_username(db, id, scraper, &stale_esse.username).await? {
            return Ok(chat);
        } else {
            warn!("聊天{}未找到, 删除该条目", {
                stale_esse.username
            });
            EsseInterestChannel::delete_by_id(stale_esse.id)
                .exec(db)
                .await?;
            continue;
        };
    }
}
