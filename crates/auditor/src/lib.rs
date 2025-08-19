use anyhow::Result;
use tokio::sync::broadcast::Receiver;
use tracing::{error, info, instrument, trace};

mod auditor_app;
mod config;
mod error;
mod namespace_auditor;

use auditor_app::AuditorApp;
use config::AuditorConfig;

#[instrument(skip_all, name = "start_auditor")]
pub async fn start(shutdown_signal: &mut Receiver<()>) -> Result<()> {
    trace!("Starting auditor application");

    let config = AuditorConfig::load()
        .map_err(|e| anyhow::anyhow!("Failed to load configuration: {}", e))?;

    info!(
        "Starting auditor with {} namespaces",
        config.namespaces.len()
    );

    let mut app = AuditorApp::from_config(config).await?;

    // Handle graceful shutdown with signal handling at the application level
    tokio::select! {
        _ = shutdown_signal.recv() => {
            info!("Shutdown signal received, initiating graceful shutdown");
            if let Err(e) = app.shutdown().await {
                error!(error = %e, "Error during shutdown");
            }
            info!("Shutdown signal sent, waiting for auditors to complete...");
        }
        result = app.run() => {
            match result {
                Ok(()) => info!("All auditors completed"),
                Err(e) => error!(error = %e, "Application error"),
            }
        }
    }

    Ok(())
}
