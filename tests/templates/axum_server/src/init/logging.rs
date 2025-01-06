pub fn setup_logging() {
    tracing_subscriber::fmt().with_max_level(tracing::Level::TRACE).init();
}