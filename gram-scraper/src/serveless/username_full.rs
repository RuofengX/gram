use anyhow::Result;
use sea_orm::{ActiveValue::Set, IntoActiveModel, QueryOrder, prelude::*};
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    scraper::Scraper,
    serveless::general::{full_info, now, resolve_username},
};
use gram_type::entity::{esse_username_full, peer_full, prelude::*};

/// 获取最老的、一天内没有解析过的、有效的full_info的esse_username表信息  
/// 如果这样的esse_username表存在, 则
///   * 解析基本信息(username -> user_id)存入user_chat
///     - 如用户名不可用, 则将该记录标记为invalid
///   * 解析其全量信息
///   * 存入user_full表
///   * 更新esse_username表中该记录的时间
///   * 返回user_full表记录id
///
/// 如不存在, 返回Ok(None)
pub async fn update_stale_esse_username(
    db: &impl ConnectionTrait,
    scraper_id: Uuid,
    scraper: &Scraper,
) -> Result<Option<Uuid>> {
    // 循环直到找到最老的esse_username表中符合筛选条件的项目
    loop {
        let stale_username = EsseUsernameFull::find()
            .order_by_asc(esse_username_full::Column::UpdatedAt)
            .filter(esse_username_full::Column::IsValid.is_null())
            .one(db)
            .await?;

        if stale_username.is_none() {
            // 没找到就返回None
            return Ok(None);
        }
        let stale_username = stale_username.unwrap();

        info!("[{}]: 开始", stale_username.username);

        // 先解析用户名
        info!("[{}]: 解析用户名", stale_username.username);
        if let Some(chat) =
            resolve_username(db, scraper_id, scraper, &stale_username.username).await?
        {
            info!("[{}]: 解析全量信息", stale_username.username);
            // 更新时间数值作为stale的参考, 让每次stale的结果都是最老的
            touch(db, stale_username.clone()).await?;
            // 标记已找到
            let mut model = stale_username.into_active_model();
            model.is_valid = Set(Some(true));
            model.update(db).await?;
            // 获取全量信息
            let ret = full_info(db, scraper, &chat).await?;
            return Ok(ret);
        } else {
            warn!("[{}]: 未找到, 标记该条目", stale_username.username);
            let mut model = stale_username.into_active_model();
            model.is_valid = Set(Some(false));
            model.update(db).await?;
            continue;
        }
    }
}

/// 更新时间数值作为stale的参考, 让每次stale的结果都是最老的
async fn touch(db: &impl ConnectionTrait, stale_username: esse_username_full::Model) -> Result<()> {
    let mut fresh_esse = stale_username.into_active_model();
    fresh_esse.updated_at = Set(now());
    fresh_esse.update(db).await?;
    Ok(())
}
