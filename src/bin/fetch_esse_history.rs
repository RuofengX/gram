use anyhow::Result;
use gram::{scraper::Scraper, serveless, signal_catch};
use sea_orm::{ConnectionTrait, TransactionTrait};
use tokio::sync::mpsc::error::TryRecvError;
use tracing::{error, info, warn};
use uuid::Uuid;

include!("../../.config.rs");

#[tokio::main]
async fn main() -> Result<()> {
    gram::init_tracing();

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

        if let Err(e) = run(&db, scraper_id, &scraper).await {
            error!("运行时出现错误: {}", e);
            break;
        }
    }

    serveless::exit_scraper(&db, scraper_id, scraper).await?;

    Ok(())
}

async fn run(
    db: &(impl ConnectionTrait + TransactionTrait),
    scraper_id: Uuid,
    scraper: &Scraper,
) -> Result<()> {
    warn!("遍历最老ESSE频道");
    let chat_id = serveless::get_stale_esse_channel(db, scraper_id, scraper).await?;

    serveless::sync_channel_history(db, scraper_id, scraper, chat_id).await?;

    // info!("睡眠30秒");
    // tokio::time::sleep(Duration::from_secs(30)).await;

    Ok(())
}
