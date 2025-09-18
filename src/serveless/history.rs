use anyhow::{Result, anyhow};
use sea_orm::{
    ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, TransactionTrait,
};
use tracing::{debug, info, instrument};
use uuid::Uuid;

use crate::{
    entity::{peer_history, prelude::PeerHistory},
    scraper::{HistoryConfig, Scraper},
    types::PackedChat,
};

/// 将数据库中的历史聊天记录向前、向后**连续**扩展  
/// latest_chunk_size控制此次扩展向后延展的条目数量  
/// 返回数组, 分别是数据库目前条数, 此次新增后向条数, 此次新增前向条数
///
/// 数据库中的历史为单块连续的记录, 同时使用事务确保数据连续、不重复
pub async fn expend_history(
    db: &impl TransactionTrait,
    scraper_id: Uuid,
    scraper: &Scraper,
    chat_id: Uuid,
    packed_chat: PackedChat,
    latest_chunk_size: usize,
) -> Result<(usize, usize, usize)> {
    let trans = db.begin().await?;
    let mut total = PeerHistory::find()
        .filter(peer_history::Column::ChatId.eq(packed_chat.0.id))
        .count(&trans)
        .await? as usize;
    if total == 0 {
        // 获取最初的记录
        total += fetch(
            &trans,
            scraper_id,
            scraper,
            chat_id,
            packed_chat,
            Some(100),
            None,
            None,
        )
        .await?;
    }

    let (old, new) = tokio::try_join!(
        expand_oldest(&trans, scraper_id, &scraper, chat_id, packed_chat, 500),
        expand_latest(&trans, scraper_id, &scraper, chat_id, packed_chat, latest_chunk_size),
    )?;

    debug!("commit transaction");
    trans.commit().await?;

    Ok((total + old + new, old,  new))
}

/// 向最新的迭代
/// 数据库中至少要有一条作为开始的参照
async fn expand_latest(
    db: &impl ConnectionTrait,
    scraper_id: Uuid,
    scraper: &Scraper,
    chat_id: Uuid,
    packed_chat: PackedChat,
    chunk_size: usize,
) -> Result<usize> {
    let mut count = 0;

    // 获取最新的history_id
    debug!("get latest history_id from db");
    let history = PeerHistory::find()
        .filter(peer_history::Column::ChatId.eq(packed_chat.0.id))
        .order_by_desc(peer_history::Column::HistoryId) // 降序就是最新的
        .one(db)
        .await?
        .ok_or(anyhow!(
            "database change (history data loss) during transaction"
        ))?;

    // 将开始消息ID向后偏移一个chunk
    let start_offset = history.history_id + chunk_size as i32;
    debug!(
        "start offset {} + {} = {}",
        history.history_id, chunk_size, start_offset
    );

    count += fetch(
        db,
        scraper_id,
        scraper,
        chat_id,
        packed_chat,
        Some(chunk_size),
        Some(start_offset),
        Some(history.history_id),
    )
    .await?;

    Ok(count)
}

/// 从最老的迭代
async fn expand_oldest(
    db: &impl ConnectionTrait,
    scraper_id: Uuid,
    scraper: &Scraper,
    chat_id: Uuid,
    packed_chat: PackedChat,
    chunk_size: usize,
) -> Result<usize> {
    let mut count = 0;

    // 获取最老的history_id
    debug!("get latest history_id from db");
    let history = PeerHistory::find()
        .filter(peer_history::Column::ChatId.eq(packed_chat.0.id))
        .order_by_asc(peer_history::Column::HistoryId) // 升序就是最老的
        .one(db)
        .await?
        .ok_or(anyhow!(
            "database change (history data loss) during transaction"
        ))?;

    // 将开始消息ID向前偏移一个chunk
    let start_offset = (history.history_id - chunk_size as i32).max(0);

    debug!(
        "start offset {} - {} = {}",
        history.history_id, chunk_size, start_offset
    );

    count += fetch(
        db,
        scraper_id,
        scraper,
        chat_id,
        packed_chat,
        Some(chunk_size),
        Some(start_offset),
        None,
    )
    .await?;

    Ok(count)
}

/// 从api获取chat的聊天记录
///
/// 如果设置了max_limit, 则将请求小于该参数的记录  
/// 如果设置了min_limit, 则所有不大于该参数的记录将被丢弃
#[instrument(level = "debug", skip(db, scraper, packed_chat))]
async fn fetch(
    db: &impl ConnectionTrait,
    scraper_id: Uuid,
    scraper: &Scraper,
    chat_id: Uuid,
    packed_chat: PackedChat,
    limit: Option<usize>,
    max_limit: Option<i32>,
    min_limit: Option<i32>,
) -> Result<usize> {
    if max_limit == Some(0){
        return Ok(0)
    }

    let packed_chat_id = packed_chat.0.id;
    let mut iter = scraper.iter_history(HistoryConfig {
        chat: packed_chat,
        limit: limit,
        offset_date: None,
        offset_id: max_limit,
    })?;
    let mut history_to_insert = Vec::new();

    while let Some(msg) = iter.next().await? {
        if let Some(min_limit) = min_limit {
            if msg.id() <= min_limit {
                // 丢弃
                debug!("drop msg({}) for min_limit({})", msg.id(), min_limit);
                continue;
            }
        }
        info!(
            "获取: ({}): {}",
            msg.id(),
            msg.text()
                .replace("\n", "\\n")
                .chars()
                .take(50)
                .collect::<String>()
        );
        let model = peer_history::ActiveModel {
            user_scraper: Set(scraper_id),
            user_chat: Set(chat_id),
            chat_id: Set(packed_chat_id),
            history_id: Set(msg.id()),
            message: Set(msg.raw.into()),
            ..Default::default()
        };
        history_to_insert.push(model);
    }

    let count = history_to_insert.len();

    debug!("insert {} history", count);
    if count > 0 {
        PeerHistory::insert_many(history_to_insert).exec(db).await?;
    }

    Ok(count)
}
