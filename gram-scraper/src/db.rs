use crate::scraper::Scraper;
use crate::serveless;
use anyhow::{Result, anyhow};
use gram_type::entity::{global_api_config, prelude::*, user_account};
use sea_orm::{ConnectionTrait, EntityTrait, TransactionTrait};
use tracing::warn;

pub async fn fetch_config(
    conn: &impl ConnectionTrait,
) -> Result<(global_api_config::Model, user_account::Model)> {
    let ret = (
        GlobalApiConfig::find()
            .one(conn)
            .await?
            .ok_or(anyhow!("no api config found in db"))?,
        UserAccount::find()
            .one(conn)
            .await?
            .ok_or(anyhow!("no user account found in db"))?,
    );
    Ok(ret)
}

pub async fn fetch_session(db: &(impl ConnectionTrait + TransactionTrait)) -> Result<(Uuid, Scraper)> {
    warn!("获取会话");
    let (scraper_id, scraper) = if let Some(ret) = serveless::scraper::resume_scraper(db).await? {
        ret
    } else {
        warn!("无可用会话，创建新会话");
        let ret = serveless::scraper::create_scraper_from_stdin(db).await?;
        ret
    };
    warn!("会话UUID: {}", scraper_id);
    Ok((scraper_id, scraper))
}
