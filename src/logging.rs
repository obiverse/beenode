use tracing_subscriber::{fmt, EnvFilter};

pub fn init_logging() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let use_json = std::env::var("BEENODE_LOG_JSON")
        .map(|value| value == "1")
        .unwrap_or(false);

    if use_json {
        let _ = fmt::Subscriber::builder()
            .with_env_filter(env_filter)
            .json()
            .with_writer(std::io::stderr)
            .try_init();
    } else {
        let _ = fmt::Subscriber::builder()
            .with_env_filter(env_filter)
            .pretty()
            .with_writer(std::io::stderr)
            .try_init();
    }
}
