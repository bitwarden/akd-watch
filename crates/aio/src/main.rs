use anyhow::Result;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let (shutdown_tx, mut shutdown_rx) = tokio::sync::broadcast::channel(1);

    // Start the auditor service in a separate task
    let auditor_handle = tokio::spawn(async move {
        if let Err(e) = akd_watch_auditor::start(&mut shutdown_rx).await {
            error!(error = ?e, "Auditor service failed");
        }
    });

    // Start the web service
    let web_handle = tokio::spawn(async {
        if let Err(e) = akd_watch_web::start().await {
            error!(error = ?e, "Web service failed");
        }
    });

    // Wait for both services to complete
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down");
            shutdown_tx.send(()).ok();
            // TODO we should probably allow for some graceful shutdown sent to the auditor and web services
        }
        _ = auditor_handle => {
            info!("Auditor service completed");
        }
        _ = web_handle => {
            info!("Web service completed");
        }
    }

    Ok(())
}
