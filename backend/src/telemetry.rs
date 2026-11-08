use tracing_subscriber::EnvFilter;

/// The levels logged when `RUST_LOG` is not set.
const DEFAULT_FILTER: &str = "chess_backend=info,tower_http=info";

/// JSON logs to stdout, one line per event.
pub fn init() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER));
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(filter)
        .init();
}
