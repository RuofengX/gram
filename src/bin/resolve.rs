use anyhow::Result;
use gram::serveless;
use tracing::{info, warn};

include!("../../.config.rs");

#[tokio::main]
async fn main() -> Result<()> {
    gram::init_tracing();

    let db = serveless::connect_db().await?;

    warn!("获取会话");
    let (id, s) = if let Some(ret) = serveless::resume_scraper(&db).await? {
        ret
    } else {
        warn!("无可用会话，创建新会话");
        let ret = serveless::create_scraper_from_stdin(&db).await?;
        ret
    };
    warn!("会话UUID: {}", id);

    let result = serveless::resolve_username(&db, id, &s, "mianbeitongjiling").await?;

    println!("{:?}", result);

    warn!("退出会话");
    serveless::exit_scraper(&db, id, s).await?;

    Ok(())
}
