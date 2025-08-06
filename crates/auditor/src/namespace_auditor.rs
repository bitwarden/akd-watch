use std::sync::Arc;
use std::time::Duration;

use akd_watch_common::{
    SerializableAuditBlobName, EpochSignature, NamespaceInfo,
    akd_configurations::verify_consecutive_append_only,
    storage::{
        AkdStorage,
        namespace_repository::NamespaceRepository,
        signing_key_repository::SigningKeyRepository,
        SignatureStorage,
    },
};
use anyhow::Result;
use tokio::sync::broadcast;
use tracing::{info, instrument, trace, warn};

/// Service responsible for auditing a single namespace
pub struct NamespaceAuditor<NR, SKR, SS> {
    namespace_info: NamespaceInfo,
    namespace_repository: Arc<NR>,
    signing_key_repository: Arc<SKR>,
    signature_storage: SS,
    sleep_duration: Duration,
    shutdown_rx: broadcast::Receiver<()>,
}

impl<NR, SKR, SS> NamespaceAuditor<NR, SKR, SS>
where
    NR: NamespaceRepository + Send + Sync + 'static,
    SKR: SigningKeyRepository + Send + Sync + 'static,
    SS: SignatureStorage + Send + Sync + 'static,
{
    pub fn new(
        namespace_info: NamespaceInfo,
        namespace_repository: Arc<NR>,
        signing_key_repository: Arc<SKR>,
        signature_storage: SS,
        sleep_duration: Duration,
        shutdown_rx: broadcast::Receiver<()>,
    ) -> Self {
        Self {
            namespace_info,
            namespace_repository,
            signing_key_repository,
            signature_storage,
            sleep_duration,
            shutdown_rx,
        }
    }

    /// Start the auditing loop for this namespace
    #[instrument(level = "info", skip_all, fields(namespace = self.namespace_info.name))]
    pub async fn run(mut self) -> Result<()> {
        info!(namespace = self.namespace_info.name, "Starting namespace auditor");

        // TODO: Check namespace status in repository before starting audit loop
        // If namespace is in failed state from previous runs, we should exit this thread immediately.

        loop {
            let should_shutdown = self.audit_cycle().await;
            if should_shutdown {
                break;
            }
        }

        info!(namespace = self.namespace_info.name, "Namespace auditor stopped");
        Ok(())
    }

    /// Run a single audit cycle and return whether shutdown was requested
    /// Returns true if shutdown was received, false otherwise
    async fn audit_cycle(&mut self) -> bool {
        match self.run_audit_cycle().await {
            Ok(_processed_count) => {
                // Always sleep after an audit cycle since poll_for_new_epochs
                // already gets all available epochs in one call
                trace!(
                    namespace = self.namespace_info.name,
                    sleep_duration = ?self.sleep_duration,
                    "Audit cycle complete, sleeping"
                );
                self.interruptible_sleep().await
            }
            Err(e) => {
                warn!(
                    namespace = self.namespace_info.name,
                    error = %e,
                    "Critical audit failure - stopping namespace auditor"
                );
                // TODO: Consider whether we should attempt recovery or permanently stop
                // For now, we stop the auditor when audit failures occur
                true // Signal shutdown
            }
        }
    }

    /// Sleep for the configured duration, but wake up immediately if shutdown is signaled
    /// Returns true if shutdown was received, false if sleep completed normally
    async fn interruptible_sleep(&mut self) -> bool {
        tokio::select! {
            _ = tokio::time::sleep(self.sleep_duration) => {
                // Sleep completed normally
                false
            }
            _ = self.shutdown_rx.recv() => {
                info!(namespace = self.namespace_info.name, "Received shutdown signal during sleep");
                true
            }
        }
    }

    /// Perform one complete audit cycle
    async fn run_audit_cycle(&mut self) -> Result<usize> {
        // Refresh namespace info from repository
        let namespace_info = self.get_fresh_namespace_info().await?;

        // Poll for new epochs
        let blob_names = self.poll_for_new_epochs(&namespace_info).await?;
        
        if !blob_names.is_empty() {
            trace!(
                namespace = namespace_info.name,
                blob_names = blob_names
                    .iter()
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
                "Found new epochs to audit"
            );
        }

        // Process each audit request
        for blob_name in &blob_names {
            if let Err(e) = self.process_audit_request(blob_name, &namespace_info).await {
                warn!(
                    namespace = namespace_info.name,
                    epoch = blob_name.epoch,
                    blob_name = blob_name.to_string(),
                    error = %e,
                    "Audit request failed - stopping further processing for this namespace"
                );
                
                // TODO: Update namespace status in repository to failed state
                // TODO: Validate namespace status before processing in future cycles
                
                // Return error to stop processing this namespace
                return Err(anyhow::anyhow!(
                    "Audit failed for epoch {} in namespace {}: {}", 
                    blob_name.epoch, 
                    namespace_info.name,
                    e
                ));
            } else {
                info!(
                    namespace = namespace_info.name,
                    epoch = blob_name.epoch,
                    blob_name = blob_name.to_string(),
                    "Successfully processed audit request"
                );
            }
        }

        Ok(blob_names.len())
    }

    /// Get fresh namespace info from the repository
    async fn get_fresh_namespace_info(&self) -> Result<NamespaceInfo> {
        self.namespace_repository
            .get_namespace_info(&self.namespace_info.name)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!("Namespace {} not found in repository", self.namespace_info.name)
            })
    }

    /// Polls the AKD for a list of unaudited epochs and returns a list of `AuditRequest`s.
    #[instrument(level = "info", skip_all, fields(namespace = namespace_info.name))]
    async fn poll_for_new_epochs(&self, namespace_info: &NamespaceInfo) -> Result<Vec<SerializableAuditBlobName>> {
        let akd = namespace_info.akd_storage();

        // get the next epoch to audit
        let mut next_epoch = self.signature_storage.latest_signed_epoch().await + 1;

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
    #[instrument(level = "info", skip_all, fields(namespace = namespace_info.name, blob_name = blob_name.to_string()))]
    async fn process_audit_request(
        &mut self,
        blob_name: &SerializableAuditBlobName,
        namespace_info: &NamespaceInfo,
    ) -> Result<()> {
        // if we've signed this epoch, skip it
        // TODO: verify this signature. if it's not valid, throw an error
        if self.signature_storage
            .get_signature(&blob_name.epoch)
            .await
            .is_some()
        {
            return Ok(());
        }

        // download the blob
        let audit_blob = namespace_info
            .akd_storage()
            .get_proof(&blob_name.into())
            .await?;
        trace!(
            namespace = namespace_info.name,
            blob_name = blob_name.to_string(),
            "Downloaded audit blob"
        );

        // decode the blob
        // TODO: do not use previous_hash, download the previous signature, verify it, and if it's missing or unverified, throw an error
        let (end_epoch, previous_hash, end_hash, proof) = audit_blob
            .decode()
            .map_err(|e| anyhow::anyhow!("Failed to decode audit blob: {:?}", e))?;

        // verify the proof
        verify_consecutive_append_only(
            &namespace_info.configuration,
            &proof,
            previous_hash,
            end_hash,
            end_epoch,
        )
        .await?;
        trace!(namespace = namespace_info.name, end_epoch, previous_hash = ?previous_hash, end_hash = ?end_hash, "Verified audit proof");

        // sign the proof
        let current_signing_key = self.signing_key_repository.get_current_signing_key().await;
        let signature = EpochSignature::sign(
            namespace_info.clone(),
            end_epoch.into(),
            end_hash,
            &current_signing_key,
        )?;
        trace!(
            namespace = namespace_info.name,
            end_epoch, "Signed audit proof"
        );

        // store the signature
        self.signature_storage
            .set_signature(end_epoch, signature)
            .await;
        trace!(
            namespace = namespace_info.name,
            end_epoch, "Stored signature for audit proof"
        );

        Ok(())
    }
}
