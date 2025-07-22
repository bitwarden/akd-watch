use std::thread;
use serde::{Deserialize, Serialize};
use tracing_subscriber;

use akd_watch_common::NamespaceInfo;

use crate::error::WatcherError;


mod error;

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
            // 2. Add to work queue for auditor
            // This is a placeholder for demonstration
            std::thread::sleep(std::time::Duration::from_secs(20));
        }
    });
}

pub async fn poll_for_new_epoch(_namespace: NamespaceInfo) -> Result<(), WatcherError> {
    // Placeholder for watching a namespace's log directory
    // 1. Poll Azure Blob Storage for new blobs
    // 2. Enqueue audit requests for new blobs
    Ok(())
}
