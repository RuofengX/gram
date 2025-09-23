use crate::{
    entity::{peer_full, prelude::*, user_chat},
    scraper::Scraper,
    types::{ApiConfig, FrozenSession, PackedChat},
};
use anyhow::anyhow;
use anyhow::{Result, bail};
use chrono::{DateTime, Local};
use sea_orm::{
    ActiveValue::{NotSet, Set},
    Condition, ConnectOptions, Database, IntoActiveModel, TransactionTrait,
    prelude::*,
};
use tracing::{debug, info, warn};

pub fn now() -> DateTimeWithTimeZone {
    let now_local: DateTime<Local> = Local::now();
    now_local.with_timezone(&now_local.offset())
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
    let mut opt = ConnectOptions::new(url);
    opt.sqlx_logging(false); // Disable SQLx log
    let db = Database::connect(opt).await?;
    Ok(db)
}

/// 将用户名解析为packe_chat并存入数据库, 返回数据库条目的uuid
///
/// 函数会自动查询同scraper之前解析的缓存, 减少FLOOD
///
/// 如任何方式都无法解析用户名, 则返回Ok(None)
pub async fn resolve_username(
    db: &impl ConnectionTrait,
    scraper_id: Uuid,
    scraper: &Scraper,
    username: &str,
) -> Result<Option<user_chat::Model>> {
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
        return Ok(Some(chat));
    }

    debug!("resolve username");
    let chat = if let Some(chat) = scraper.resolve_username(&username).await? {
        chat
    } else {
        info!("未找到用户名: {}", username);
        return Ok(None);
    };

    // 存入user_chat
    debug!("insert user_chat");
    let chat = user_chat::ActiveModel {
        id: NotSet,
        updated_at: NotSet,
        user_scraper: Set(scraper_id),
        username: Set(Some(username.to_string())),
        user_id: Set(chat.0.id),
        packed_chat: Set(chat),
        joined: Set(false),
    }
    .insert(db)
    .await?;

    Ok(Some(chat))
}

pub async fn join_channel(
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

pub async fn quit_channel(
    db: &impl TransactionTrait,
    scraper_id: Uuid,
    scraper: &Scraper,
    chat_id: Uuid,
) -> Result<()> {
    info!("退出群组");
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

/// 获取user_chat表中(已经解析好的)该用户/频道的全量数据
///
/// 保存到peer_full中并返回peer_full的id
pub async fn full_info(
    db: &impl ConnectionTrait,
    scraper: &Scraper,
    user_chat: &user_chat::Model,
) -> Result<Uuid> {
    // 查询peer_full作为缓存返回
    debug!("search peer_full");
    if let Some(chat) = PeerFull::find()
        .filter(user_chat::Column::UserId.eq(user_chat.id))
        .one(db)
        .await?
    {
        return Ok(chat.id);
    }

    let username = user_chat.username.clone();
    let chat = user_chat.packed_chat;

    // 对端为用户
    if chat.0.is_user() {
        let full = scraper.fetch_user_info(chat).await?;
        let ret = peer_full::ActiveModel {
            id: NotSet,
            updated_at: NotSet,
            user_chat: Set(user_chat.id),
            user_id: Set(user_chat.user_id),
            username: Set(username),
            user_full: Set(Some(full)),
            channel_full: Set(None),
        }
        .insert(db)
        .await?;
        return Ok(ret.id);
    }

    // 对端为频道
    if chat.0.is_channel() {
        let full = scraper.fetch_channel_info(chat).await?;
        let ret = peer_full::ActiveModel {
            id: NotSet,
            updated_at: NotSet,
            user_chat: Set(user_chat.id),
            user_id: Set(user_chat.user_id),
            username: Set(username),
            user_full: Set(None),
            channel_full: Set(Some(full)),
        }
        .insert(db)
        .await?;
        return Ok(ret.id);
    }

    // 对端为其他东西
    bail!("cannot fetch full info of chat");
}
