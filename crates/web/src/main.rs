use std::collections::HashMap;

use akd_watch_common::storage::{
    namespaces::NamespaceStorage, signatures::SignatureStorage, signing_keys::VerifyingKeyStorage,
};
use axum::Router;
use tokio::net::TcpListener;
use tracing::{error, info, trace};
use tracing_subscriber;

use crate::web_config::WebConfig;

mod error;
mod routes;
mod web_config;

#[derive(Clone)]
pub(crate) struct AppState {
    namespace_storage: NamespaceStorage,
    signature_storage: HashMap<String, SignatureStorage>,
    verifying_key_storage: VerifyingKeyStorage,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();
    trace!("Starting web server");

    // Load configuration
    let config = WebConfig::load().expect("Failed to load configuration");
    match config.validate() {
        Ok(_) => info!("Web configuration is valid"),
        Err(e) => {
            error!(error = %e, "Invalid web configuration");
            std::process::exit(1);
        }
    }
    info!("Starting web server with configuration: {:?}", config);

    // Initialize application state
    let namespace_storage = config.namespace_storage.build_namespace_storage();
    let signature_storage = config
        .signature_storage
        .build_signature_storage(&namespace_storage)
        .await
        .expect("Failed to initialize signature storage");
    let verifying_key_storage = config
        .signing
        .build_verifying_key_storage()
        .expect("Failed to initialize verifying key storage");
    let app_state = AppState {
        namespace_storage,
        signature_storage,
        verifying_key_storage,
    };

    // Build API
    let app = Router::new()
        .merge(routes::api_routes())
        .with_state(app_state);

    // Start server
    let addr = config.socket_addr();
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Listening on http://{}", addr);
    axum::serve(listener, app).await.unwrap();
}
