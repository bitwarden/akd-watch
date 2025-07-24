use akd_watch_common::SerializableAuditBlobName;
use anyhow::Result;
use tracing::{instrument, trace};
use tracing_subscriber;

use akd_watch_common::{
    AuditVersion, EpochSignature, NamespaceInfo, NamespaceStatus,
    configurations::{AkdConfiguration, verify_consecutive_append_only},
    crypto::SigningKey,
    storage::{
        AkdStorage, InMemoryStorage, SignatureStorage,
        whatsapp_akd_storage::WhatsAppAkdStorage,
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
        log_directory: Some("logs/example_namespace".to_string()),
        last_verified_epoch: None,
        status: NamespaceStatus::Online,
        signature_version: AuditVersion::One,
    }];
    let namespaces = infos
        .into_iter()
        .map(|info| Namespace {
            info,
            akd_storage: WhatsAppAkdStorage::new(),
            signature_storage: InMemoryStorage::new(),
        })
        .collect::<Vec<_>>();

    // TODO: load from configuration
    let sleep_time = std::time::Duration::from_secs(20);
    // TODO: load from configuration
    let signing_key = SigningKey::generate();

    for namespace in namespaces {
        let mut namespace = namespace.clone();
        let signing_key = signing_key.clone();
        tokio::spawn(async move {
            loop {
                // query for new epochs
                let blob_names = match poll_for_new_epochs(namespace.clone()).await {
                    Ok(audit_requests) => {
                        trace!(
                            namespace = namespace.info.name,
                            "Polled for new epochs successfully"
                        );
                        audit_requests
                    }
                    Err(e) => {
                        trace!(namespace = namespace.info.name, error = %e, "Failed to poll for new epochs");
                        tokio::time::sleep(sleep_time).await;
                        continue;
                    }
                };

                // audit all the new epochs
                for blob_name in &blob_names {
                    match process_audit_request(blob_name, &mut namespace, &signing_key).await {
                        Ok(_) => trace!(
                            namespace = namespace.info.name,
                            epoch = blob_name.epoch,
                            blob_name = blob_name.to_string(),
                            "Processed audit request successfully"
                        ),
                        Err(e) => {
                            trace!(namespace = namespace.info.name, epoch = blob_name.epoch, blob_name = blob_name.to_string(), error = %e, "Error processing audit request")
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
struct Namespace<A: AkdStorage, S: SignatureStorage> {
    pub info: NamespaceInfo,
    pub akd_storage: A,
    pub signature_storage: S,
}

/// Polls the AKD for a list of unaudited epochs and returns a list of `AuditRequest`s.
#[instrument(level = "trace", skip_all, fields(namespace = namespace.info.name))]
async fn poll_for_new_epochs<A: AkdStorage, S: SignatureStorage>(
    namespace: Namespace<A, S>,
) -> Result<Vec<SerializableAuditBlobName>> {
    let akd = namespace.akd_storage;
    let signatures = namespace.signature_storage;

    // get the latest signed epoch
    let mut last_known_epoch = signatures.latest_signed_epoch().await;

    // Check if the namespace has a proof for the latest epoch
    let mut result = Vec::new();
    loop {
        if akd.has_proof(last_known_epoch).await {
            trace!(akd = %akd, epoch = last_known_epoch, "AKD has published a new proof");

            if let Ok(proof_name) = akd.get_proof_name(last_known_epoch).await {
                // Add the proof name to the queue
                trace!(akd = %akd, epoch = last_known_epoch, proof_name = proof_name.to_string(), "Retrieved proof name");
                result.push(proof_name.into());
                // increment the epoch and continue to check for the next one
                last_known_epoch += 1;
                continue;
            } else {
                trace!(akd = %akd, epoch = last_known_epoch, "Failed to retrieve proof name for epoch");
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
#[instrument(level = "trace", skip_all, fields(namespace = namespace.info.name, blob_name = blob_name.to_string()))]
async fn process_audit_request(
    blob_name: &SerializableAuditBlobName,
    namespace: &mut Namespace<impl AkdStorage, impl SignatureStorage>,
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
    let audit_blob = namespace.akd_storage.get_proof(&blob_name.into()).await?;

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

    // sign the proof
    let signature = EpochSignature::sign(
        namespace.info.clone(),
        end_epoch.into(),
        end_hash,
        &mut signing_key.clone(),
    )?;

    // store the signature
    namespace
        .signature_storage
        .set_signature(end_epoch, signature)
        .await;

    Ok(())
}
