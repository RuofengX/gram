use gram::{scraper::Scraper, types::FreezeSession};

include!("../../.config.rs");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let freeze = FreezeSession::load("./test.session")?;

    let client = Scraper::from_frozen(freeze).await?;

    let this = client.check_self().await?;

    println!("access_hash: {:?}", this.access_hash);

    let freeze = client.freeze();
    freeze.dump("./test.session")?;

    Ok(())
}
