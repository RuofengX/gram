use anyhow::Result;
use gram::{init_tracing, scraper::Scraper};

include!("../../.config.rs");

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let frozen = serde_json::from_str(include_str!("../../session.json"))?;

    let s = Scraper::from_frozen(frozen, &TEST_CONFIG.into()).await?;
    let ping_self = s.get_self().await?;

    println!("{}", serde_json::to_string_pretty(&ping_self)?);

    // test resolve name
    let user = s.resolve_username("MTGBG").await?;

    println!("{}", serde_json::to_string_pretty(&user)?);

    Ok(())
}
