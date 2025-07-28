use std::collections::HashMap;

use akd_watch_common::{
    SerializableAuditBlobName,
    storage::namespace_repository::{ InMemoryNamespaceRepository, NamespaceRepository},
};
use anyhow::Result;
use tracing::{info, instrument, trace, warn};
use tracing_subscriber;

use akd_watch_common::{
     EpochSignature, NamespaceInfo, NamespaceStatus,
    configurations::{AkdConfiguration, verify_consecutive_append_only},
    crypto::SigningKey,
    storage::{
        AkdStorage, InMemoryStorage, SignatureStorage,
    },
};

mod error;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // TODO: load namespaces from configuration
    let infos = vec![NamespaceInfo {
        configuration: AkdConfiguration::WhatsAppV1Configuration,
        name: "example_namespace".to_string(),
        log_directory: "logs/example_namespace".to_string(),
        last_verified_epoch: 0.into(),
        status: NamespaceStatus::Online,
    }];
    let namespace_repository = init_namespace_repository(&infos).await;
    let signature_storage = infos
        .iter()
        .fold(HashMap::new(), |mut agg, item| {
            // TODO: Init storage for each namespace from configuration
            agg.insert(item.name.clone(), InMemoryStorage::new());
            agg
        });

    // TODO: load from configuration
    let sleep_time = std::time::Duration::from_secs(20);
    // TODO: load from configuration
    let signing_key = SigningKey::generate();

    for namespace in infos {
        let signing_key = signing_key.clone();
        let namespace_repository = namespace_repository.clone();
        tokio::spawn(async move {
            loop {
                // query for latest namespace info
                let namespace_info = namespace_repository
                    .get_namespace_info(&namespace.name)
                    .await
                    .expect(&format!(
                        "Failed to get namespace info for {}",
                        namespace.name
                    ))
                    .expect(&format!(
                        "Namespace {} not found in repository",
                        namespace.name
                    ));
                let mut namespace = Namespace {
                    info: namespace_info,
                    // TODO: this storage isn't doing anything, we need to send this between loops, loaded from configuration
                    signature_storage: InMemoryStorage::new(),
                };
                // query for new epochs
                let blob_names = match poll_for_new_epochs(namespace.clone()).await {
                    Ok(audit_requests) => {
                        trace!(
                            namespace = namespace.info.name,
                            blob_names = audit_requests
                                .iter()
                                .map(|n| n.to_string())
                                .collect::<Vec<_>>()
                                .join(", "),
                            "Polled for new epochs successfully"
                        );
                        audit_requests
                    }
                    Err(e) => {
                        warn!(namespace = namespace.info.name, error = %e, "Failed to poll for new epochs");
                        tokio::time::sleep(sleep_time).await;
                        continue;
                    }
                };

                // audit all the new epochs
                for blob_name in &blob_names {
                    match process_audit_request(blob_name, &mut namespace, &signing_key).await {
                        Ok(_) => info!(
                            namespace = namespace.info.name,
                            epoch = blob_name.epoch,
                            blob_name = blob_name.to_string(),
                            "Processed audit request successfully"
                        ),
                        Err(e) => {
                            warn!(namespace = namespace.info.name, epoch = blob_name.epoch, blob_name = blob_name.to_string(), error = %e, "Error processing audit request")
                        }
                    }
                }

                // if we processed some requests, we don't need to sleep
                if blob_names.is_empty() {
                    trace!(namespace = namespace.info.name, sleep_time = ?sleep_time, "Sleeping for next poll");
                    tokio::time::sleep(sleep_time).await;
                }
            }
        });
    }
}

#[derive(Clone, Debug)]
struct Namespace<S: SignatureStorage> {
    pub info: NamespaceInfo,
    pub signature_storage: S,
}

/// Polls the AKD for a list of unaudited epochs and returns a list of `AuditRequest`s.
#[instrument(level = "info", skip_all, fields(namespace = namespace.info.name))]
async fn poll_for_new_epochs<S: SignatureStorage>(
    namespace: Namespace<S>,
) -> Result<Vec<SerializableAuditBlobName>> {
    let akd = namespace.info.akd_storage();
    let signatures = namespace.signature_storage;

    // get the latest signed epoch
    let mut last_known_epoch = signatures.latest_signed_epoch().await;

    // Check if the namespace has a proof for the latest epoch
    let mut result = Vec::new();
    loop {
        if akd.has_proof(last_known_epoch).await {
            info!(akd = %akd, epoch = last_known_epoch, "AKD has published a new proof");

            if let Ok(proof_name) = akd.get_proof_name(last_known_epoch).await {
                // Add the proof name to the queue
                info!(akd = %akd, epoch = last_known_epoch, proof_name = proof_name.to_string(), "Retrieved proof name");
                result.push(proof_name.into());
                // increment the epoch and continue to check for the next one
                last_known_epoch += 1;
                continue;
            } else {
                warn!(akd = %akd, epoch = last_known_epoch, "Failed to retrieve proof name for epoch");
                break;
            }
        } else {
            trace!(akd = %akd, epoch = last_known_epoch, "AKD has not published a proof for this epoch, yet");
            break;
        }
    }

    Ok(result)
}

/// Downloads the audit proof for the given `AuditRequest`, verifies it, and stores the signature if successful.
#[instrument(level = "info", skip_all, fields(namespace = namespace.info.name, blob_name = blob_name.to_string()))]
async fn process_audit_request(
    blob_name: &SerializableAuditBlobName,
    namespace: &mut Namespace<impl SignatureStorage>,
    signing_key: &SigningKey,
) -> Result<()> {
    // if we've signed this epoch, skip it
    if namespace
        .signature_storage
        .get_signature(&blob_name.epoch)
        .await
        .is_some()
    {
        return Ok(());
    }

    // download the blob
    let audit_blob = namespace
        .info
        .akd_storage()
        .get_proof(&blob_name.into())
        .await?;
    trace!(
        namespace = namespace.info.name,
        blob_name = blob_name.to_string(),
        "Downloaded audit blob"
    );

    // decode the blob
    let (end_epoch, previous_hash, end_hash, proof) = audit_blob
        .decode()
        .map_err(|e| anyhow::anyhow!("Failed to decode audit blob: {:?}", e))?;

    // verify the proof
    verify_consecutive_append_only(
        &namespace.info.configuration,
        &proof,
        previous_hash,
        end_hash,
        end_epoch,
    )
    .await?;
    trace!(namespace = namespace.info.name, end_epoch, previous_hash = ?previous_hash, end_hash = ?end_hash, "Verified audit proof");

    // sign the proof
    let signature = EpochSignature::sign(
        namespace.info.clone(),
        end_epoch.into(),
        end_hash,
        &mut signing_key.clone(),
    )?;
    trace!(
        namespace = namespace.info.name,
        end_epoch, "Signed audit proof"
    );

    // store the signature
    namespace
        .signature_storage
        .set_signature(end_epoch, signature)
        .await;
    trace!(
        namespace = namespace.info.name,
        end_epoch, "Stored signature for audit proof"
    );

    Ok(())
}

#[instrument(level = "info", skip_all)]
async fn init_namespace_repository(
    configured_namespaces: &Vec<NamespaceInfo>,
) -> InMemoryNamespaceRepository {
    let mut namespace_repository = InMemoryNamespaceRepository::new();

    let namespaces = namespace_repository
        .list_namespaces()
        .await
        .unwrap_or_default();
    // Ensure namespaces are in the repository
    for namespace in configured_namespaces {
        if namespaces.iter().any(|n| n.name == namespace.name) {
            info!(namespace = ?namespace, "Namespace already exists in repository, skipping");
            continue;
        }
        info!(namespace = ?namespace, "Adding namespace to repository");
        namespace_repository
            .add_namespace(namespace.clone())
            .await
            .unwrap();
    }

    namespace_repository
}
