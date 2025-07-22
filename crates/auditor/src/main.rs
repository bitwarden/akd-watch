use akd::local_auditing::AuditBlob;
use anyhow::{Result, anyhow};
use tracing_subscriber;

use akd_watch_common::{
    configurations::verify_consecutive_append_only, crypto::SigningKey, storage::{AuditRequestQueue, InMemoryQueue, InMemoryStorage, SignatureStorage}, AuditRequest, EpochSignature
};

mod error;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();


    let storage = InMemoryStorage::new();
    let mut queue = InMemoryQueue::new();

    // TODO: Replace with actual signing key retrieval
    let signing_key = SigningKey::generate();

    println!("Listening for audit requests.");
    while let Some(audit_request) = queue.dequeue().await {
        let storage = storage.clone();
        let secret_key = signing_key.clone();
        // spawn a thread to handle each audit request
        tokio::spawn(async move {
            // Process the audit request
            match process_audit_request(audit_request, storage.clone(), secret_key).await {
                Ok(result) => {
                    println!("Audit result: {:?}", result);
                    // TODO: Store the result
                }
                Err(e) => {
                    eprintln!("Error processing audit request: {:?}", e);
                }
            };
            return;
        });
    }
    println!("Stopped listening for audit requests.");
}

async fn process_audit_request(
    request: AuditRequest,
    mut storage: impl SignatureStorage,
    mut signing_key: SigningKey,
) -> Result<()> {
    let blob_name = request.parse_blob_name()?;

    // download the blob
    // TODO: lookup namespace url
    let audit_blob = get_proof("http://example.com/blobs", &request).await?;
    // Note we ignore start_hash because we want to lock it to our previous signature
    let (end_epoch, _, end_hash, proof) =
        audit_blob.decode().map_err(|err| anyhow!("{:?}", err))?;

    // get the previous signature
    let Some(previous_signature) = storage.get_signature(&blob_name.epoch).await else {
        return Err(anyhow!(
            "Unable to find signature for epoch {}",
            blob_name.epoch
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

pub(crate) async fn get_proof(url: &str, request: &AuditRequest) -> Result<AuditBlob> {
    let name = request.parse_blob_name()?;
    let url = format!("{}/{}", url, request.blob_name);
    let resp = reqwest::get(url).await?.bytes().await?;
    let data = resp.to_vec();

    Ok(AuditBlob { data, name })
}
