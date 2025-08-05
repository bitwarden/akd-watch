use anyhow::Result;
use tracing::info;
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
    app.run().await
}


