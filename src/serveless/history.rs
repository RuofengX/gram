use anyhow::{Result, anyhow};
use sea_orm::{
    ActiveValue::Set, ColumnTrait, Condition, ConnectionTrait, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, TransactionTrait,
};
use tracing::{debug, info, instrument};
use uuid::Uuid;

use crate::{
    entity::{peer_history, prelude::PeerHistory},
    scraper::{HistoryConfig, Scraper},
    types::PackedChat,
};

/// 将数据库中的历史聊天记录向前、向后**连续**扩展  
/// 返回数组, 分别是数据库目前条数和此次新增条数
///
/// 数据库中的历史为单块连续的记录, 同时使用事务确保数据连续、不重复
pub async fn expend_history(
    db: &impl TransactionTrait,
    scraper_id: Uuid,
    scraper: &Scraper,
    chat_id: Uuid,
    packed_chat: PackedChat,
    chunk_size: usize,
) -> Result<(usize, usize)> {
    let trans = db.begin().await?;
    let mut total = PeerHistory::find()
        .filter(Condition::all().add(peer_history::Column::UserChat.eq(chat_id)))
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
        )
        .await?;
    }

    let (old, new) = tokio::try_join!(
        expand_oldest(
            &trans,
            scraper_id,
            &scraper,
            chat_id,
            packed_chat,
            chunk_size
        ),
        expand_latest(
            &trans,
            scraper_id,
            &scraper,
            chat_id,
            packed_chat,
            chunk_size
        ),
    )?;

    trans.commit().await?;

    Ok((total + old + new, old + new))
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
        .filter(peer_history::Column::UserChat.eq(chat_id))
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
        .filter(peer_history::Column::UserChat.eq(chat_id))
        .order_by_asc(peer_history::Column::HistoryId) // 升序就是最老的
        .one(db)
        .await?
        .ok_or(anyhow!(
            "database change (history data loss) during transaction"
        ))?;

    // 将开始消息ID向前偏移一个chunk
    let start_offset = history.history_id - chunk_size as i32;
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
    )
    .await?;

    Ok(count)
}

#[instrument(level = "debug", skip(db, scraper, packed_chat))]
async fn fetch(
    db: &impl ConnectionTrait,
    scraper_id: Uuid,
    scraper: &Scraper,
    chat_id: Uuid,
    packed_chat: PackedChat,
    limit: Option<usize>,
    offset_id: Option<i32>,
) -> Result<usize> {
    let mut iter = scraper.iter_history(HistoryConfig {
        chat: packed_chat,
        limit: limit,
        offset_date: None,
        offset_id: offset_id,
    })?;
    let mut history_to_insert = Vec::new();

    while let Some(msg) = iter.next().await? {
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
            history_id: Set(msg.id()),
            message: Set(msg.raw.into()),
            ..Default::default()
        };
        history_to_insert.push(model);
    }

    let count = history_to_insert.len();

    debug!("insert {} history", count);
    PeerHistory::insert_many(history_to_insert).exec(db).await?;

    Ok(count)
}
