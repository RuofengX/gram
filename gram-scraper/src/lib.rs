use std::io::BufRead;

use tracing::warn;

pub mod db;
pub mod executor;
pub mod scraper;
pub mod serve;
pub mod serveless;

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
