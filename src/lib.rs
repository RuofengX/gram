use std::io::BufRead;

pub mod scraper;
pub mod types;
pub mod serve;
pub mod executor;
mod test;

pub fn stdin_read_line(prompt: &'static str) -> tokio::sync::oneshot::Receiver<String> {
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
