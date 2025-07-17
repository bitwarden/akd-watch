use akd::local_auditing::AuditBlob;
use anyhow::{Result, anyhow};
use futures_util::stream::StreamExt;
use serde::{Deserialize, Serialize};
use tracing_subscriber;

use akd_watch_common::{
    configurations::verify_consecutive_append_only, storage::InMemoryStorage, AuditRequest, SignatureStorage
};

mod error;
// use crate::error::AuditorError;

// Placeholder for audit result type
#[derive(Clone, Serialize, Deserialize, Debug)]
struct AuditResult {
    blob_name: String,
    verified: bool,
    signature: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Initialize multiplexed Redis connection
    let client = redis::Client::open("redis://127.0.0.1/").expect("Failed to create Redis client");
    let mut conn = client
        .get_async_pubsub()
        .await
        .expect("Failed to get Redis connection");

    // Subscribe to audit requests channel
    conn.subscribe(&["audit_requests"])
        .await
        .expect("Failed to subscribe to audit_requests channel");

    let storage = InMemoryStorage::new();

    println!("Listening for audit requests.");
    while let Some(msg) = conn.on_message().next().await {
        let storage = storage.clone();
        // spawn a thread to handle each audit request
        tokio::spawn(async move {
            let Ok(audit_request) = AuditRequest::try_from(msg) else {
                eprintln!("Failed to parse audit request from message");
                return;
            };

            // Process the audit request
            match process_audit_request(audit_request, storage.clone()).await {
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
    storage: impl SignatureStorage,
) -> Result<AuditResult> {
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
        end_epoch,
        end_hash,
    );

    // Placeholder for processing an audit request
    // 2. Verify the audit proof
    // 3. Return the result
    Ok(AuditResult {
        blob_name: blob_name.to_string(),
        verified: true, // Placeholder value
        signature: "placeholder_signature".to_string(),
    })
}

pub(crate) async fn get_proof(url: &str, request: &AuditRequest) -> Result<AuditBlob> {
    let name = request.parse_blob_name()?;
    let url = format!("{}/{}", url, request.blob_name);
    let resp = reqwest::get(url).await?.bytes().await?;
    let data = resp.to_vec();

    Ok(AuditBlob { data, name })
}
