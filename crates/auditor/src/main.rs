use akd_watch_auditor::start;
use tracing::{error, info};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
    .with_max_level(tracing::Level::INFO)
    .init();

    let (shutdown_tx, mut shutdown_rx) = tokio::sync::broadcast::channel(1);

    let handle = start(&mut shutdown_rx);

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down");
            shutdown_tx.send(()).ok();
        }
        result = handle => {
            if let Err(e) = result {
                error!(error = %e, "Application error");
                std::process::exit(1);
            }
        }
    }
}


