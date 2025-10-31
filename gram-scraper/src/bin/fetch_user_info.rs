use anyhow::Result;
use gram_scraper::db::fetch_session;
use gram_scraper::{serveless, signal_catch};
use tokio::sync::mpsc::error::TryRecvError;
use tracing::{debug, error, warn};

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

        match serveless::username_full::update_stale_esse_username(&db, scraper_id, &scraper).await {
            Ok(Some(id)) => {
                debug!("fetch and insert username info to {}", id);
                continue;
            }
            Ok(None) => {
                warn!("已无待更新项目");
                break;
            }
            Err(e) => {
                error!("运行时出现错误: {}", e);
                break;
            }
        }
    }

    serveless::scraper::exit_scraper(&db, scraper_id, scraper).await?;

    Ok(())
}
