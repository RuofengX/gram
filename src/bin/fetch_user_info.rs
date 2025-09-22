use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    gram::init_tracing();

    Ok(())
}