use tracing::{instrument, trace};
use tracing_subscriber;

use akd_watch_common::{configurations::AkdConfiguration, storage::{whatsapp_akd_storage::WhatsAppAkdStorage, AkdStorage, AuditRequestQueue, InMemoryQueue, InMemoryStorage, SignatureStorage}, AuditRequest, AuditVersion, NamespaceInfo, NamespaceStatus};

use crate::error::WatcherError;

mod error;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // TODO: load namespaces from configuration
    let infos = vec![
        NamespaceInfo {
            configuration: AkdConfiguration::WhatsAppV1Configuration,
            name: "example_namespace".to_string(),
            log_directory: Some("logs/example_namespace".to_string()),
            last_verified_epoch: None,
            status: NamespaceStatus::Online,
            signature_version: AuditVersion::One,
        },
    ];
    let namespaces = infos
        .into_iter()
        .map(|info| {
            Namespace {
                info,
                akd_storage: WhatsAppAkdStorage::new(),
                signature_storage: InMemoryStorage::new(),
            }
        })
        .collect::<Vec<_>>();
    let queue = InMemoryQueue::new();

    // TODO: load from configuration
    let sleep_time = std::time::Duration::from_secs(20);

    // Spawn watcher threads for each namespace
    for namespace in namespaces {
        let queue = queue.clone();
        tokio::spawn(async move {
            loop {
                match poll_for_new_epoch(namespace.clone(), queue.clone()).await {
                    Ok(_) => trace!(namespace = namespace.info.name, "Watcher completed successfully"),
                    Err(e) => trace!(namespace = namespace.info.name, error = %e, "Watcher encountered an error"),
                };
                tokio::time::sleep(sleep_time).await;
            };
        });
    }
}

#[instrument(level = "trace", skip_all, fields(namespace = namespace.info.name))]
async fn poll_for_new_epoch<A: AkdStorage, S: SignatureStorage, Q: AuditRequestQueue>(namespace: Namespace<A, S>, mut queue: Q) -> Result<(), WatcherError> {
    let akd = namespace.akd_storage;
    let signatures = namespace.signature_storage;
    let latest_epoch = signatures.latest_signed_epoch().await;

    // get the latest signed epoch
    let mut latest_epoch = latest_epoch;
    // Check if the namespace has a proof for the latest epoch
    let mut to_enqueue = Vec::new();
    loop {
        if akd.has_proof(latest_epoch).await {
            trace!(akd = %akd, epoch = latest_epoch, "AKD has published a new proof");

            if let Ok(proof_name) = akd.get_proof_name(latest_epoch).await {
                // Add the proof name to the queue
                trace!(akd = %akd, epoch = latest_epoch, proof_name = proof_name.to_string(), "Retrieved proof name");
                to_enqueue.push(AuditRequest::new(namespace.info.clone(), proof_name.to_string()));
                // increment the epoch and continue to check for the next one
                latest_epoch += 1;
                continue;
            } else {
                trace!(akd = %akd, epoch = latest_epoch, "Failed to retrieve proof name for epoch");
                break;
            }
        } else {
            trace!(akd = %akd, epoch = latest_epoch, "AKD has not published a proof for this epoch, yet");
            break;
        }
    }

    // Enqueue all collected audit requests
    if !to_enqueue.is_empty() {
        trace!(akd = %akd, epoch = latest_epoch, "Enqueuing audit requests");
        queue.enqueue_n(to_enqueue).await;
    } else {
        trace!(akd = %akd, epoch = latest_epoch, "No new audit requests to enqueue");
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct Namespace<A: AkdStorage, S: SignatureStorage> {
    pub info: NamespaceInfo,
    pub akd_storage: A,
    pub signature_storage: S,
}
