use gram::{scraper::Scraper, types::FrozenSession};

include!("../../.config.rs");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let freeze = FrozenSession::load("./test.session")?;

    let api_config = &TEST_CONFIG.into();
    let client = Scraper::from_frozen(freeze, api_config).await?;

    let this = client.get_self().await?;

    println!("access_hash: {:?}", this.access_hash);

    let freeze = client.freeze();
    freeze.dump("./test.session")?;

    Ok(())
}
