use anyhow::{Result, anyhow};
use tracing_subscriber;

use akd_watch_common::{
    configurations::verify_consecutive_append_only, crypto::SigningKey, storage::{whatsapp_akd_storage::WhatsAppAkdStorage, AkdStorage, AuditRequestQueue, InMemoryQueue, InMemoryStorage, SignatureStorage}, AuditRequest, EpochSignature
};

mod error;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();


    let storage = InMemoryStorage::new();
    let mut queue = InMemoryQueue::new();
    // TODO: replace with namepaced akd list
    let akd = WhatsAppAkdStorage::new();

    // TODO: Replace with actual signing key retrieval
    let signing_key = SigningKey::generate();

    println!("Listening for audit requests.");
    while let Some(audit_request) = queue.dequeue().await {
        let storage = storage.clone();
        let secret_key = signing_key.clone();
        let akd = akd.clone();
        // spawn a thread to handle each audit request
        tokio::spawn(async move {
            // Process the audit request
            match process_audit_request(audit_request.clone(), storage, secret_key, akd).await {
                Ok(_) => {
                    println!("Processed audit request successfully for request {:?}", audit_request);
                },
                Err(e) => {
                    eprintln!("Error processing audit request: {:?}", e);
                }
            };
        });
    }
    println!("Stopped listening for audit requests.");
}

async fn process_audit_request(
    request: AuditRequest,
    mut storage: impl SignatureStorage,
    mut signing_key: SigningKey,
    akd: impl AkdStorage,
) -> Result<()> {
    let blob_name = request.parse_blob_name()?;

    // if we've signed this epoch, skip it
    if storage.get_signature(&blob_name.epoch).await.is_some() {
        return Ok(());
    }

    // download the blob
    // TODO: lookup namespace url
    let audit_blob = akd.get_proof(&blob_name).await?;
    // Note we ignore start_hash because we want to tie it to previously verified audits, so we
    // download the signature for the previous epoch
    let (end_epoch, _, end_hash, proof) =
        audit_blob.decode().map_err(|err| anyhow!("{:?}", err))?;
    let previous_epoch = blob_name.epoch - 1;

    // get the previous signature
    let missing_epochs = missing_previous_signatures(storage.clone(), previous_epoch).await?;
    if !missing_epochs.is_empty() {
        // TODO: Enqueue the missing epochs. This will heal signature storage for this range
        return Err(anyhow!(
            "Missing previous signatures for epochs: {:?}",
            missing_epochs
        ));
    }
    let Some(previous_signature) = storage.get_signature(&previous_epoch).await else {
        return Err(anyhow!(
            "Unable to find signature for previous epoch: {}",
            previous_epoch
        ));
    };

    // Verify the audit proof
    _ = verify_consecutive_append_only(
        &request.namespace.configuration,
        &proof,
        previous_signature.epoch_root_hash()?,
        end_hash,
        end_epoch,
    )
    .await?;

    // Generate an epoch signature
    let signature = EpochSignature::sign(
        request.namespace,
        end_epoch.into(),
        end_hash,
        &mut signing_key,
    )?;

    // Store the signature
    storage.set_signature(blob_name.epoch, signature).await;

    Ok(())
}

async fn missing_previous_signatures(
    storage: impl SignatureStorage,
    mut epoch: u64,
) -> Result<Vec<u64>> {
    let mut missing_epochs = Vec::new();
    loop {
        if epoch == 0 {
            break;
        }
        if storage.has_signature(&epoch).await {
            break;
        }
        missing_epochs.push(epoch);
        epoch -= 1;
    }
    missing_epochs.reverse();
    Ok(missing_epochs)
}
