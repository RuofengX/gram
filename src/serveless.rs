use crate::{
    entity::{
        esse_interest_channel, peer_history,
        prelude::{
            EsseInterestChannel, GlobalApiConfig, UserAccount, UserChat, UserScraper,
            VUserChatWithId,
        },
        user_chat, user_scraper,
    },
    scraper::{HistoryConfig, Scraper},
    stdin_read_line,
    types::{ApiConfig, FrozenSession, PackedChat},
};
use anyhow::Result;
use anyhow::anyhow;
use chrono::{DateTime, Local};
use sea_orm::{
    ActiveValue::{NotSet, Set},
    Condition, Database, IntoActiveModel, QueryOrder, TransactionTrait,
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
    scraper_id: Uuid,
    scraper: &Scraper,
) -> Result<Vec<PackedChat>> {
    info!("枚举当前聊天并发送到数据库");
    let mut ret = Vec::new();
    let mut user_chat_to_insert = Vec::new();

    for (username, chat) in scraper.list_chats_with_username().await? {
        ret.push(chat.clone());
        user_chat_to_insert.push(user_chat::ActiveModel {
            user_scraper: Set(scraper_id),
            packed_chat: Set(chat),
            username: Set(username),
            ..Default::default()
        });
    }
    // 将数据插入user_chat表
    user_chat::Entity::insert_many(user_chat_to_insert)
        .exec(db)
        .await?;

    Ok(ret)
}

pub async fn exit_scraper(
    db: &impl ConnectionTrait,
    scraper_id: Uuid,
    scraper: Scraper,
) -> Result<()> {
    info!("退出爬虫，更新数据库状态");

    debug!("freeze scraper instence");
    let frozen = scraper.freeze();
    let scraper_update = user_scraper::ActiveModel {
        id: Set(scraper_id),
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
    scraper_id: Uuid,
    scraper: &Scraper,
    username: &str,
) -> Result<Option<Uuid>> {
    // 查询user_chat作为缓存返回
    debug!("search user_chat");
    if let Some(chat) = UserChat::find()
        .filter(
            Condition::all()
                // 不同的scraper的access_hash不通用, 无法跨scraper缓存
                .add(user_chat::Column::UserScraper.eq(scraper_id))
                .add(user_chat::Column::Username.eq(username)),
        )
        .one(db)
        .await?
    {
        return Ok(Some(chat.id));
    }

    debug!("resolve username");
    let chat = if let Some(chat) = scraper.resolve_username(&username).await? {
        chat
    } else {
        info!("未找到用户名: {}", username);
        return Ok(None);
    };

    // 存入user_chat
    debug!("update db");
    let chat = user_chat::ActiveModel {
        user_scraper: Set(scraper_id),
        username: Set(Some(username.to_string())),
        packed_chat: Set(chat),
        ..Default::default()
    }
    .insert(db)
    .await?
    .id;

    Ok(Some(chat))
}

pub async fn get_stale_esse_channel(
    db: &impl ConnectionTrait,
    scraper_id: Uuid,
    scraper: &Scraper,
) -> Result<Uuid> {
    // 循环直到找到最老的esse对应的packed_chat
    loop {
        let stale_esse = EsseInterestChannel::find()
            .order_by_asc(esse_interest_channel::Column::UpdatedAt) // TODO: 需确认最老的是正序排列
            .one(db)
            .await?
            .ok_or(anyhow!("esse channel not found"))?;
        if let Some(chat) = resolve_username(db, scraper_id, scraper, &stale_esse.username).await? {
            debug!("update db");
            // 更新时间数值作为stale的参考, 让每次stale的结果都是最老的
            let mut fresh_esse = stale_esse.into_active_model();
            fresh_esse.updated_at = Set(now());
            fresh_esse.update(db).await?;
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

pub async fn sync_channel_history(
    db: &(impl ConnectionTrait + TransactionTrait),
    scraper_id: Uuid,
    scraper: &Scraper,
    chat_id: Uuid,
) -> Result<()> {
    debug!("get packed_chat from db");
    let chat = VUserChatWithId::find_by_id(chat_id)
        .one(db)
        .await?
        .ok_or(anyhow!("chat not found"))?;

    let chat = if chat.joined {
        chat.packed_chat
    } else {
        debug!("join channel");
        join_channel(db, scraper_id, scraper, chat.id).await?
    };

    // 分析已有历史

    debug!("create history iterator");
    let mut i = scraper.iter_history(HistoryConfig {
        chat,
        limit: Some(10),
        offset_date: None,
        offset_id: Some(72637), // 倒序
    })?;

    debug!("iterate history to insert");
    let mut history_to_insert = Vec::new();
    while let Some(msg) = i.next().await? {
        let model = peer_history::ActiveModel {
            user_scraper: Set(scraper_id),
            user_chat: Set(chat_id),
            history_id: Set(msg.id()),
            message: Set(msg.raw.into()),
            ..Default::default()
        };
        history_to_insert.push(model);
    }

    debug!("insert ({}) history", history_to_insert.len());
    peer_history::Entity::insert_many(history_to_insert)
        .exec(db)
        .await?;

    // TODO: 如何一个账号channel太多，退出该channel并将db的packed_chat设置为null
    quit_channel(db, scraper_id, scraper, chat_id).await?;

    return Ok(());
}

async fn join_channel(
    db: &impl TransactionTrait,
    scraper_id: Uuid,
    scraper: &Scraper,
    chat_id: Uuid,
) -> Result<PackedChat> {
    debug!("transaction start");
    let trans = db.begin().await?;

    // 检查自身是否加入
    debug!("check if self already joined");
    let chat = VUserChatWithId::find_by_id(chat_id)
        .one(&trans)
        .await?
        .ok_or(anyhow!("user_chat not found"))?;

    if chat.joined {
        info!("已加入, 忽略");
        return Ok(chat.packed_chat);
    }

    // 加入群组
    debug!("join channel");
    let live_chat = scraper
        .join_chat(chat.packed_chat)
        .await?
        .ok_or(anyhow!("join return none"))?;

    debug!("update db");
    let mut chat = chat.into_active_model();
    chat.updated_at = Set(now());
    chat.user_scraper = Set(scraper_id);
    chat.packed_chat = Set(live_chat.pack().into());
    chat.joined = Set(true);
    let chat = chat.update(&trans).await?;

    debug!("transaction commit");
    trans.commit().await?;

    Ok(chat.packed_chat)
}

async fn quit_channel(
    db: &impl TransactionTrait,
    scraper_id: Uuid,
    scraper: &Scraper,
    chat_id: Uuid,
) -> Result<()> {
    debug!("transaction start");
    let trans = db.begin().await?;

    // 检查自身是否退出
    debug!("check if self already quit");
    let chat = VUserChatWithId::find_by_id(chat_id)
        .one(&trans)
        .await?
        .ok_or(anyhow!("user_chat not found"))?;

    if !chat.joined {
        info!("已退出, 忽略");
        return Ok(());
    }

    // 退出群组
    debug!("quit channel");
    scraper.quit_chat(chat.packed_chat).await?;

    debug!("update db");
    let mut chat = chat.into_active_model();
    chat.updated_at = Set(now());
    chat.user_scraper = Set(scraper_id);
    chat.joined = Set(false);
    chat.update(&trans).await?;

    debug!("transaction commit");
    trans.commit().await?;

    Ok(())
}
