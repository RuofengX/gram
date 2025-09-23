use anyhow::Result;
use gram_scraper::{serveless, signal_catch};
use tokio::sync::mpsc::error::TryRecvError;
use tracing::{error, info, warn};

include!("../../.config.rs");

#[tokio::main]
async fn main() -> Result<()> {
    gram_scraper::init_tracing();

    let db = serveless::general::connect_db().await?;

    warn!("获取会话");
    let (scraper_id, scraper) = if let Some(ret) = serveless::scraper::resume_scraper(&db).await? {
        ret
    } else {
        warn!("无可用会话，创建新会话");
        let ret = serveless::scraper::create_scraper_from_stdin(&db).await?;
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

        if let Err(e) =
            serveless::username_full::update_stale_esse_usename(&db, scraper_id, &scraper).await
        {
            error!("运行时出现错误: {}", e);
            break;
        }
    }

    serveless::scraper::exit_scraper(&db, scraper_id, scraper).await?;

    Ok(())
}
