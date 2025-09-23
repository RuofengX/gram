use std::{io::BufRead, sync::Once};

use tracing::warn;

pub mod db;
pub mod entity;
pub mod executor;
pub mod scraper;
pub mod serve;
pub mod serveless;
mod test;
pub mod types;

pub fn stdin_read_line(prompt: String) -> tokio::sync::oneshot::Receiver<String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    tokio::task::spawn_blocking(move || {
        let mut stdin = std::io::stdin().lock();
        let mut buffer = String::new();
        println!("{}", prompt);
        stdin.read_line(&mut buffer).expect("stdin wont broken");
        println!("(confirmed)");
        tx.send(buffer).expect("channel should not close");
    });
    rx
}

static LOG_INIT: Once = Once::new();

#[cfg(not(debug_assertions))]
pub fn init_tracing() {
    LOG_INIT.call_once(|| {
        use tracing::Level;

        tracing_subscriber::fmt()
            .with_max_level(Level::INFO)
            .with_target(false)
            .init();
    });
}

#[cfg(debug_assertions)]
pub fn init_tracing() {
    LOG_INIT.call_once(|| {
        use tracing::Level;

        tracing_subscriber::fmt()
            .with_file(true)
            .with_line_number(true)
            .with_thread_names(true)
            .with_max_level(Level::DEBUG)
            .init();
    });
}

pub fn signal_catch() -> tokio::sync::mpsc::Receiver<()> {
    let (tx, rx) = tokio::sync::mpsc::channel::<()>(1);

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen for ctrlc");
        warn!("收到 Ctrl+C, 进程即将退出");
        let _ = tx.send(()).await;
    });

    rx
}
