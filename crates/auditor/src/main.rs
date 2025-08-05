use std::collections::HashMap;

use akd_watch_common::{
    SerializableAuditBlobName,
    storage::namespace_repository::{ InMemoryNamespaceRepository, NamespaceRepository},
};
use anyhow::Result;
use tracing::{info, instrument, trace, warn};
use tracing_subscriber;

use akd_watch_common::{
     EpochSignature, NamespaceInfo,
    akd_configurations::verify_consecutive_append_only,
    crypto::SigningKey,
    storage::{
        AkdStorage, InMemoryStorage, SignatureStorage,
    },
};

mod error;
mod config;

use config::AuditorConfig;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Load configuration
    let config = AuditorConfig::load()
        .expect("Failed to load configuration. Please check your config file or environment variables.");
    
    info!("Loaded configuration with {} namespaces", config.namespaces.len());

    // Initialize namespace repository and convert configs to namespace infos
    let (namespace_repository, infos) = init_namespace_repository(&config.namespaces).await?;
    let signature_storage = infos
        .iter()
        .fold(HashMap::new(), |mut agg, item| {
            // TODO: Init storage for each namespace from configuration
            agg.insert(item.name.clone(), InMemoryStorage::new());
            agg
        });

    // Load sleep time from configuration
    let sleep_time = config.sleep_duration();
    
    // TODO: Implement proper signing key management:
    // - Store key_id next to keys in the keyfile
    // - Support key rotation with current and past keys stored in the keyfile
    // - Add config for key lifetime and forced rotation
    let signing_key = SigningKey::generate();

    for namespace in infos {
        let signing_key = signing_key.clone();
        let namespace_repository = namespace_repository.clone();
        let signature_storage = signature_storage.get(&namespace.name).expect("missing signature storage").clone();
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
                    signature_storage: signature_storage.clone(),
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
    
    Ok(())
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

    // get the next epoch to audit
    let mut next_epoch = signatures.latest_signed_epoch().await + 1;

    // Check if the namespace has a proof for the next epoch
    let mut result = Vec::new();
    loop {
        if akd.has_proof(next_epoch).await {
            info!(akd = %akd, epoch = next_epoch, "AKD has published a new proof");

            if let Ok(proof_name) = akd.get_proof_name(next_epoch).await {
                // Add the proof name to the queue
                info!(akd = %akd, epoch = next_epoch, proof_name = proof_name.to_string(), "Retrieved proof name");
                result.push(proof_name.into());
                // increment the epoch and continue to check for the next one
                next_epoch += 1;
                continue;
            } else {
                warn!(akd = %akd, epoch = next_epoch, "Failed to retrieve proof name for epoch");
                break;
            }
        } else {
            trace!(akd = %akd, epoch = next_epoch, "AKD has not published a proof for this epoch, yet");
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
    namespace_configs: &[config::NamespaceConfig],
) -> Result<(InMemoryNamespaceRepository, Vec<NamespaceInfo>)> {
    let mut namespace_repository = InMemoryNamespaceRepository::new();

    let existing_namespaces = namespace_repository
        .list_namespaces()
        .await
        .unwrap_or_default();

    let mut infos = Vec::new();
    
    for ns_config in namespace_configs {
        // Check if namespace already exists in the repository
        let existing_info = existing_namespaces
            .iter()
            .find(|info| info.name == ns_config.name);
        
        // Convert config to namespace info, preserving existing last_verified_epoch if available
        let (namespace_info, status_changed) = ns_config.to_namespace_info(existing_info)
            .map_err(|e| anyhow::anyhow!("Configuration error for namespace {}: {}", ns_config.name, e))?;
        
        // Add to repository if it doesn't exist, or update if status changed
        if existing_info.is_none() {
            info!(namespace = ?namespace_info, "Adding new namespace to repository");
            namespace_repository
                .add_namespace(namespace_info.clone())
                .await
                .map_err(|e| anyhow::anyhow!("Failed to add namespace {}: {}", ns_config.name, e))?;
        } else if status_changed {
            info!(namespace = ns_config.name, old_status = ?existing_info.unwrap().status, new_status = ?namespace_info.status, "Updating namespace status in repository");
            namespace_repository
                .update_namespace(namespace_info.clone())
                .await
                .map_err(|e| anyhow::anyhow!("Failed to update namespace {}: {}", ns_config.name, e))?;
        } else {
            info!(namespace = ns_config.name, "Using existing namespace from repository (no changes)");
        }
        
        infos.push(namespace_info);
    }

    Ok((namespace_repository, infos))
}
