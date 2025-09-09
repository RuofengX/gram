use crate::types::{ApiConfig, FrozenSession};
use anyhow::Result;
use tokio::sync::oneshot;

use crate::scraper::Scraper;

pub async fn login_async(
    api_config: ApiConfig,
    phone: String,
    code: oneshot::Receiver<String>,
) -> Result<FrozenSession> {
    let scraper = Scraper::new(&api_config).await?;
    let _u = scraper.login_async(&phone, code).await?;
    Ok(scraper.freeze())
}

pub async fn activate_frozen_with<R>(
    api_config: ApiConfig,
    frozen: FrozenSession,
    with: impl Fn(&Scraper) -> Result<R>,
) -> Result<R> {
    let scraper = Scraper::from_frozen(frozen, &api_config).await?;
    with(&scraper)
}

pub async fn list_channel(api_config: ApiConfig, frozen: FrozenSession) {
    todo!()
}
