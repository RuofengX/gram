pub mod prelude;

pub mod esse_interest_channel;
pub mod esse_username_full;
pub mod global_api_config;
pub mod peer_file_part;
pub mod peer_full;
pub mod peer_history;
pub mod peer_media;
pub mod peer_participant;
pub mod user_account;
pub mod user_chat;
pub mod user_scraper;
pub mod v_user_chat_with_id;

pub use sea_orm::ActiveValue::NotSet;
pub use sea_orm::ActiveValue::Set;

pub async fn connect_db() -> anyhow::Result<sea_orm::DatabaseConnection> {
    tracing::warn!("连接到数据库");
    dotenv::dotenv().unwrap();
    let url = dotenv::var("DATABASE_URL".to_owned())?;

    let mut opt = sea_orm::ConnectOptions::new(url);

    #[cfg(not(debug_assertions))]
    // release: Disable SQLx log
    opt.sqlx_logging(false);

    #[cfg(debug_assertions)]
    opt.sqlx_logging(true);

    let db = sea_orm::Database::connect(opt).await?;
    Ok(db)
}
