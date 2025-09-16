use gram::{executor::Executor, serve::app};
use tracing::warn;

include!("../../.config.rs");

#[tokio::main]
async fn main() {
    gram::init_tracing();

    let executor = Executor::new(TEST_API_CONFIG.into());

    let bind = "[::]:1170";
    let listener = tokio::net::TcpListener::bind(bind).await.unwrap();

    warn!("server start at {}", bind);

    axum::serve(listener, app(executor)).await.unwrap();
}
