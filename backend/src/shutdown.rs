use tokio::signal;

/// Resolves on Ctrl-C or SIGTERM, so requests in flight finish before the server stops.
pub async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("the Ctrl-C handler installs");
    };
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("the SIGTERM handler installs")
            .recv()
            .await;
    };
    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    tracing::info!("shutting down");
}
