use gram::scraper::Scraper;

include!("../../.config.rs");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let freeze = serde_json::from_str(include_str!("../../session.json"))?;

    let client = Scraper::from_freeze(freeze, TEST_CONFIG.into()).await?;

    println!("{:?}", client.check_self().await?);

    Ok(())
}
