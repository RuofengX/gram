use std::sync::Once;

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
