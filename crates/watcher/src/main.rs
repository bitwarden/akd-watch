use std::thread;
use tracing::{trace, trace_span};
use tracing_subscriber;

use akd_watch_common::{storage::{whatsapp_akd_storage::WhatsAppAkdStorage, AkdStorage, InMemoryStorage, SignatureStorage}, NamespaceInfo};

use crate::error::WatcherError;

mod error;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // TODO: load namespaces from configuration
    let storages = vec![NamespaceStorage(WhatsAppAkdStorage::new(), InMemoryStorage::new())];

    // TODO: load from configuration
    let sleep_time = std::time::Duration::from_secs(20);

    // Spawn watcher threads for each namespace
    for storage in storages {
        let epoch = storage.1.latest_signed_epoch().await.expect("Failed to get latest signed epoch");
        let namespace = storage.0.clone();
        tokio::spawn(async move {
            let span = trace_span!("polling_namespace", namespace = %namespace, epoch = epoch);
            let _guard = span.enter();
            if namespace.has_proof(epoch).await {
                trace!(namespace = %namespace, epoch = epoch, "Queried for epoch, found proof");
                // enqueue audit request and check for the n+1 epoch
            } else {
                trace!(namespace = %namespace, epoch = epoch, "Queried for epoch, but it was not found");
            }
            // _guard guard will exit the span automatically here
        });
    }
}

pub async fn poll_for_new_epoch<A: AkdStorage, S: SignatureStorage>(storage: NamespaceStorage<A, S>) -> Result<(), WatcherError> {
    let span = trace_span!("poll_for_new_epoch", namespace = %storage.0, epoch = storage.1.latest_signed_epoch().await.unwrap_or(0));
    let _guard = span.enter();

    // Poll the namespace's storage for another epoch
    

    // Placeholder for watching a namespace's log directory
    // 1. Poll Azure Blob Storage for new blobs
    // 2. Enqueue audit requests for new blobs
    Ok(())
}

struct NamespaceStorage<A: AkdStorage,S: SignatureStorage>(A,S);
