use std::net::SocketAddr;
use std::thread;
use axum::Router;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tracing_subscriber;

mod web;

// Placeholder for audit result type
#[derive(Clone, Serialize, Deserialize)]
struct AuditResult {
    blob_name: String,
    verified: bool,
    signature: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Spawn watcher thread
    thread::spawn(move || {
        // TODO: Replace with async runtime if needed
        loop {
            // 1. Poll Azure Blob Storage for new blobs
            // 2. Download new blob
            // 3. Run AKD audit proof verification
            // 4. Store result in watcher_results
            // This is a placeholder for demonstration
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    });
    println!("Watcher thread started.");

    // Build API
    let app = Router::new().merge(web::api_routes());

    // Start server
    // TODO: Make address configurable
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Listening on http://{}", addr);
    axum::serve(listener, app.into_make_service()).await.unwrap();
}
