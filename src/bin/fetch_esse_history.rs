use anyhow::Result;
use gram::serveless;
use tracing::{info, warn};

include!("../../.config.rs");

#[tokio::main]
async fn main() -> Result<()> {
    gram::init_tracing();

    let db = serveless::connect_db().await?;

    info!("获取会话");
    let s = if let Some(s) = serveless::resume_scraper(&db).await? {
        s
    } else {
        info!("无可用会话，创建新会话");
        let (uuid, s) = serveless::create_scraper_from_stdin(&db).await?;
        warn!("会话UUID: {}", uuid);
        s
    };

    let ping_result = s.get_self().await?;

    println!("{}", serde_json::to_string(&ping_result)?);

    Ok(())
}
