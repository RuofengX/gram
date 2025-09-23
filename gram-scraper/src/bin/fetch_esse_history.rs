use anyhow::Result;
use gram_scraper::{scraper::Scraper, serveless::{self, channel_history::update_stale_esse_channel}, signal_catch};
use sea_orm::{ConnectionTrait, TransactionTrait};
use tokio::sync::mpsc::error::TryRecvError;
use tracing::{error, info, warn};
use uuid::Uuid;

include!("../../.config.rs");

#[tokio::main]
async fn main() -> Result<()> {
    gram_scraper::init_tracing();

    let db = serveless::connect_db().await?;

    warn!("获取会话");
    let (scraper_id, scraper) = if let Some(ret) = serveless::resume_scraper(&db).await? {
        ret
    } else {
        warn!("无可用会话，创建新会话");
        let ret = serveless::create_scraper_from_stdin(&db).await?;
        ret
    };
    warn!("会话UUID: {}", scraper_id);

    info!("开始监听ctrl+c");
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

    serveless::exit_scraper(&db, scraper_id, scraper).await?;

    Ok(())
}
