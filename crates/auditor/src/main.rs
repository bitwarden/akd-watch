use anyhow::Result;
use tracing::{info, error};
use tracing_subscriber;

mod error;
mod config;
mod auditor_app;
mod namespace_auditor;

use config::AuditorConfig;
use auditor_app::AuditorApp;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = AuditorConfig::load()
        .map_err(|e| anyhow::anyhow!("Failed to load configuration: {}", e))?;
    
    info!("Starting auditor with {} namespaces", config.namespaces.len());

    let app = AuditorApp::from_config(config).await?;

    // Handle graceful shutdown with signal handling at the application level
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received Ctrl+C, initiating graceful shutdown");
            if let Err(e) = app.shutdown() {
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


