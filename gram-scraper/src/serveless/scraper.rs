use crate::{
    entity::{prelude::*, user_scraper},
    scraper::Scraper,
    types::{ApiConfig, FrozenSession},
};
use anyhow::{Result, anyhow};
use sea_orm::{ActiveValue::Set, IntoActiveModel, QueryOrder, TransactionTrait, prelude::*};
use tokio::sync::oneshot;
use tracing::{debug, info, warn};
use uuid::Uuid;

pub async fn login_async(
    api_config: ApiConfig,
    phone: String,
    code: oneshot::Receiver<String>,
) -> Result<FrozenSession> {
    let scraper = Scraper::login_async(api_config, &phone, code).await?;
    Ok(scraper.freeze())
}

pub async fn resume_scraper(db: &impl TransactionTrait) -> Result<Option<(Uuid, Scraper)>> {
    info!("获取冻结会话");
    let trans = db.begin().await?;
    let ret = if let Some(scraper) = UserScraper::find()
        .filter(user_scraper::Column::InUse.eq(false)) // 选择未在使用的会话
        .order_by_desc(user_scraper::Column::UpdatedAt) // 始终选择最新的会话
        .one(&trans)
        .await?
    {
        debug!("{:?}", scraper.frozen_session);

        debug!("get relate api_config");
        let api_config = scraper
            .find_related(GlobalApiConfig)
            .one(&trans)
            .await?
            .ok_or(anyhow!("relate global_api_config not found"))?;
        debug!("{:?}", api_config);

        debug!("create scraper instance");
        info!("尝试启用会话: {}", scraper.id);
        let s = Scraper::unfreeze(scraper.frozen_session.clone(), api_config.into()).await?;

        debug!("set db scraper state");
        let mut scraper = scraper.into_active_model();
        scraper.in_use = Set(true);
        let id = scraper.update(&trans).await?.id;
        trans.commit().await?;

        Some((id, s))
    } else {
        warn!("未找到暂停的会话");
        None
    };
    Ok(ret)
}
