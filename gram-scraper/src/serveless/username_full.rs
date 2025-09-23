use anyhow::{Result, anyhow};
use sea_orm::{ActiveValue::Set, IntoActiveModel, QueryOrder, prelude::*};
use tracing::warn;
use uuid::Uuid;

use crate::{
    entity::{esse_username_full, prelude::*},
    scraper::Scraper,
    serveless::general::{full_info, now, resolve_username},
};

/// 获取最老的、没有解析过full_info的esse_username的packed_chat信息  
/// peer_full表中已经有的不再解析, 类似缓存
///
/// 返回user_chat表的id
pub async fn update_stale_esse_usename(
    db: &impl ConnectionTrait,
    scraper_id: Uuid,
    scraper: &Scraper,
) -> Result<Uuid> {
    // 循环直到找到最老的esse_username表中有效的项目
    loop {
        let stale_username = EsseUsernameFull::find()
            .order_by_asc(esse_username_full::Column::UpdatedAt) // 时间的ASCending顺序排序第一个就是最老的
            .one(db)
            .await?
            .ok_or(anyhow!("esse username table empty"))?;

        // 先解析用户名
        if let Some(chat) =
            resolve_username(db, scraper_id, scraper, &stale_username.username).await?
        {
            // 获取全量信息
            let ret = full_info(db, scraper, &chat).await?;
            // 更新时间数值作为stale的参考, 让每次stale的结果都是最老的
            let mut fresh_esse = stale_username.into_active_model();
            fresh_esse.updated_at = Set(now());
            fresh_esse.update(db).await?;
            return Ok(ret);
        } else {
            warn!("username[{}]未找到, 删除该条目", {
                stale_username.username
            });
            EsseInterestChannel::delete_by_id(stale_username.id)
                .exec(db)
                .await?;
            continue;
        }
    }
}
