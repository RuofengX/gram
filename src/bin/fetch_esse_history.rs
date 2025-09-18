use anyhow::Result;
use gram::serveless;
use tracing::{info, warn};

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

    warn!("同步聊天列表");
    let chat_list = serveless::sync_chat(&db, scraper_id, &scraper).await?;
    for c in chat_list {
        info!("{}", serde_json::to_string(&c)?);
    }

    warn!("遍历最老ESSE群组");
    let chat_id = serveless::get_stale_esse_channel(&db, scraper_id, &scraper).await?;
    println!("{:?}", chat_id);

    warn!("获取群组历史记录");
    serveless::sync_channel_history(&db, scraper_id, &scraper, chat_id).await?;

    warn!("退出会话");
    serveless::exit_scraper(&db, scraper_id, scraper).await?;

    Ok(())
}
