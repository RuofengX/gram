use super::general::join_channel;
use crate::entity::{esse_interest_channel, user_chat};
use crate::serveless::general::{now, quit_channel, resolve_username};
use crate::serveless::history::expend_history;
use crate::{entity::prelude::*, scraper::Scraper};
use anyhow::Result;
use anyhow::anyhow;
use sea_orm::ActiveValue::Set;
use sea_orm::{IntoActiveModel, QueryOrder, TransactionTrait, prelude::*};
use tracing::{debug, warn};

/// 获取最久未更新的esse频道在user_chat表中的uuid
///
/// 函数内部由[`resolve_username`]保证缓存充分利用
pub async fn update_stale_esse_channel(
    db: &(impl ConnectionTrait + TransactionTrait),
    scraper_id: Uuid,
    scraper: &Scraper,
) -> Result<Uuid> {
    // 循环直到找到最老的esse对应的packed_chat
    loop {
        let stale_esse = EsseInterestChannel::find()
            .order_by_asc(esse_interest_channel::Column::UpdatedAt) // 时间的ASCending顺序排序第一个就是最老的
            .one(db)
            .await?
            .ok_or(anyhow!("esse channel table empty"))?;
        if let Some(chat) = resolve_username(db, scraper_id, scraper, &stale_esse.username).await? {
            // 更新历史记录
            sync_channel_history(db, scraper_id, scraper, &chat).await?;
            // 更新时间数值作为stale的参考, 让每次stale的结果都是最老的
            let mut fresh_esse = stale_esse.into_active_model();
            fresh_esse.updated_at = Set(now());
            fresh_esse.update(db).await?;
            return Ok(chat.id);
        } else {
            warn!("username[{}]未找到, 删除该条目", {
                stale_esse.username
            });
            EsseInterestChannel::delete_by_id(stale_esse.id)
                .exec(db)
                .await?;
            continue;
        };
    }
}

/// 同步频道历史到数据库peer_history
async fn sync_channel_history(
    db: &(impl ConnectionTrait + TransactionTrait),
    scraper_id: Uuid,
    scraper: &Scraper,
    user_chat: &user_chat::Model,
) -> Result<()> {
    warn!(
        "开始: 频道[{}]历史记录",
        user_chat.username.clone().unwrap_or("-".to_string())
    );
    let chat = if user_chat.joined {
        user_chat.packed_chat
    } else {
        debug!("join channel");
        join_channel(db, scraper_id, scraper, user_chat.id).await?
    };

    // 分析已有历史
    // 保持数据库中记录永远连续, 仅使用向前追溯、向后增加

    debug!("start expand history");

    let mut all = 0;
    let mut latest_chunk_size = 50;
    loop {
        let (total, old, new) =
            expend_history(db, scraper_id, &scraper, &user_chat, latest_chunk_size).await?;
        all += old + new;
        warn!(
            "迭代: 频道({}) - 原:{}/历史:{}/更新:{}",
            chat.0.id, total, old, new
        );
        match (old, new) {
            (0, 0) => break, // 历史迭代完毕, 期间前向没有新增, 直接退出
            (0, _) => {
                // 历史迭代完毕, 有新增仍在迭代
                latest_chunk_size = 500; // 最大速度迭代新增
            }
            (0.., 0) => {
                // 历史没有迭代完毕, 新增迭代完毕
                latest_chunk_size = 1; // 最小速度迭代新增
            }
            (0.., 0..50) => {
                // 历史没有迭代完毕, 新增不足50
                latest_chunk_size = new; // 下次新增获取量减少, 至这次的新增量
            }
            (0.., 50) => continue, // 历史没有迭代完毕, 且新增500个, 保持当前速度迭代新增
            _ => unreachable!(),
        }
    }
    warn!("总计: 频道({}) - {}", chat.0.id, all);

    debug!("quit channel");
    quit_channel(db, scraper_id, scraper, user_chat.id).await?;

    return Ok(());
}
