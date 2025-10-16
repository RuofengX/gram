use anyhow::Result;
use gram_scraper::db::fetch_session;
use gram_scraper::{
    serveless::{self, channel_history::update_stale_esse_channel},
    signal_catch,
};
use tokio::sync::mpsc::error::TryRecvError;
use tracing::error;

include!("../../.config.rs");

#[tokio::main]
async fn main() -> Result<()> {
    gram_core::log::init_tracing();

    let db = gram_type::entity::connect_db().await?;

    let (scraper_id, scraper) = fetch_session(&db).await?;

    let mut rx = signal_catch();
    loop {
        if !matches!(rx.try_recv(), Err(TryRecvError::Empty)) {
            // ctrl+c
            break;
        }

        if let Err(e) = update_stale_esse_channel(&db, scraper_id, &scraper).await {
            error!("运行时出现错误: {}", e);
            break;
        }
    }

    serveless::scraper::exit_scraper(&db, scraper_id, scraper).await?;

    Ok(())
}
