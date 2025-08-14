use akd_watch_common::{NamespaceStatus, timed_event};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use akd_watch_common::{
    EpochSignature, NamespaceInfo, SerializableAuditBlobName,
    akd_configurations::verify_consecutive_append_only,
    akd_storage_factory::AkdStorageFactory,
    storage::{
        AkdStorage, namespaces::NamespaceRepository, signatures::SignatureRepository,
        signing_keys::SigningKeyRepository,
    },
};
use anyhow::Result;
use tokio::sync::broadcast::Receiver;
use tracing::{debug, error, info, instrument, trace, warn};

use crate::error::AuditError;

const MAX_EPOCHS_PER_POLL: usize = 50;

/// Service responsible for auditing a single namespace
pub struct NamespaceAuditor<NR, SKR, SS> {
    namespace_name: String,
    namespace_repository: Arc<RwLock<NR>>,
    signing_key_repository: Arc<RwLock<SKR>>,
    signature_storage: SS,
    sleep_duration: Duration,
    shutdown_rx: Receiver<()>,
}

impl<NR, SKR, SS> NamespaceAuditor<NR, SKR, SS>
where
    NR: NamespaceRepository + Send + Sync + 'static,
    SKR: SigningKeyRepository + Send + Sync + 'static,
    SS: SignatureRepository + Send + Sync + 'static,
{
    pub fn new(
        namespace_info: NamespaceInfo,
        namespace_repository: Arc<RwLock<NR>>,
        signing_key_repository: Arc<RwLock<SKR>>,
        signature_storage: SS,
        sleep_duration: Duration,
        shutdown_rx: Receiver<()>,
    ) -> Self {
        Self {
            namespace_name: namespace_info.name.clone(),
            namespace_repository,
            signing_key_repository,
            signature_storage,
            sleep_duration,
            shutdown_rx,
        }
    }

    /// Start the auditing loop for this namespace
    #[instrument(level = "info", skip_all, fields(namespace = self.namespace_name))]
    pub async fn run(mut self) -> Result<()> {
        // TODO: Check namespace status in repository before starting audit loop
        // If namespace is in failed state from previous runs, we should exit this thread immediately.

        loop {
            let should_shutdown = self.audit_cycle().await;
            if should_shutdown {
                break;
            }
        }

        info!(namespace = ?self.namespace_name, "Namespace auditor stopped");
        Ok(())
    }

    /// Run a single audit cycle and return whether shutdown was requested
    /// Returns true if shutdown was received, false otherwise
    async fn audit_cycle(&mut self) -> bool {
        match self.run_audit_cycle().await {
            Ok(processed_count) => {
                // Always sleep after an audit cycle since poll_for_new_epochs
                // already gets all available epochs in one call
                trace!(
                    namespace = self.namespace_name,
                    sleep_duration = ?self.sleep_duration,
                    processed_count,
                    "Audit cycle complete"
                );

                self.interruptible_sleep(&processed_count).await
            }
            Err(e) => {
                warn!(
                    namespace = self.namespace_name,
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
    async fn interruptible_sleep(&mut self, processed_count: &usize) -> bool {
        let sleep_duration = if *processed_count != MAX_EPOCHS_PER_POLL {
            debug!(
                namespace = self.namespace_name,
                sleep_duration = ?self.sleep_duration,
                "Sleeping for configured duration after processing epochs"
            );
            self.sleep_duration
        } else {
            debug!(
                namespace = self.namespace_name,
                "Processed all epochs in this cycle, no sleep needed"
            );
            Duration::from_millis(10) // No sleep if we processed all epochs, but we want to check for shutdown
        };

        match interruptible_sleep(sleep_duration, &mut self.shutdown_rx).await {
            true => {
                info!(
                    namespace = self.namespace_name,
                    "Received shutdown signal during sleep"
                );
                true // Signal shutdown
            }
            false => {
                trace!(
                    namespace = self.namespace_name,
                    "Sleep completed normally"
                );
                false // Sleep completed without shutdown
            }
        }
    }

    /// Perform one complete audit cycle
    async fn run_audit_cycle(&mut self) -> Result<usize> {
        // Refresh namespace info from repository
        let namespace_info = self.get_fresh_namespace_info().await?;
        trace!(
            namespace = ?namespace_info,
            "Running audit cycle for namespace"
        );

        // Refuse to audit if the namespace is disabled or in a failed state
        if !namespace_info.status.is_active() {
            warn!(
                namespace = namespace_info.name,
                status = ?namespace_info.status,
                "Namespace is not online, but is running audits."
            );

            return Err(anyhow::anyhow!(
                "Namespace {} is not online (status: {:?})",
                namespace_info.name,
                namespace_info.status
            ));
        }

        // Poll for new epochs
        let blob_names = self.poll_for_new_epochs(&namespace_info).await?;
        info!(
            namespace = namespace_info.name,
            new_epochs = ?(blob_names.iter().map(|b| b.epoch).collect::<Vec<_>>()),
            count = blob_names.len(),
            "Polled for new epochs"
        );

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
            let process_future = timed_event!(with_result(res) INFO, self.process_audit_request(blob_name, &namespace_info);
                    namespace = namespace_info.name,
                    epoch = blob_name.epoch,
                    success = res.is_ok(),
                    blob_name = blob_name.to_string(), "Processed audit request");
            if let Err(e) = process_future.await {
                // We're stopping further processing anyway
                if let Err(e) = self
                    .handle_audit_failure(&namespace_info, blob_name, &e)
                    .await
                {
                    error!(
                        namespace = namespace_info.name,
                        epoch = blob_name.epoch,
                        blob_name = blob_name.to_string(),
                        error = %e,
                        "Failed to handle audit failure"
                    );
                }

                // Return error to stop processing this namespace
                return Err(anyhow::anyhow!(
                    "Audit failed for epoch {} in namespace {}: {}",
                    blob_name.epoch,
                    namespace_info.name,
                    e
                ));
            } else {
                // record the successful audit
                let mut repo = self.namespace_repository.write().await;
                repo.update_namespace(
                    namespace_info.update_last_verified_epoch(blob_name.epoch.into()),
                )
                .await?;
            }
        }

        Ok(blob_names.len())
    }

    async fn handle_audit_failure(
        &self,
        namespace_info: &NamespaceInfo,
        blob_name: &SerializableAuditBlobName,
        error: &AuditError,
    ) -> Result<(), AuditError> {
        trace!(
            namespace = self.namespace_name,
            error = %error,
            "Handling audit failure"
        );

        match error {
            AuditError::SignatureNotFound(epoch) => {
                error!(
                namespace = namespace_info.name,
                epoch = %epoch,
                "Signature not found for epoch - this may indicate a gap in the audit chain"
                );
                // Update namespace to indicate signature storage failure, not AKD failure
                let mut repo = self.namespace_repository.write().await;
                repo.update_namespace(namespace_info.update_status(NamespaceStatus::SignatureLost))
                    .await?;
            }
            _ => {
                error!(
                    namespace = namespace_info.name,
                    epoch = blob_name.epoch,
                    blob_name = blob_name.to_string(),
                    error = %error,
                    "Audit request failed - stopping further processing for this namespace"
                );
                // Update namespace status to indicate failure
                let mut repo = self.namespace_repository.write().await;
                repo.update_namespace(
                    namespace_info.update_status(NamespaceStatus::SignatureVerificationFailed),
                )
                .await?;
            }
        };
        Ok(())
    }

    /// Get fresh namespace info from the repository
    async fn get_fresh_namespace_info(&self) -> Result<NamespaceInfo> {
        let repo = self.namespace_repository.read().await;
        repo.get_namespace_info(&self.namespace_name)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Namespace {} not found in repository",
                    self.namespace_name
                )
            })
    }

    /// Polls the AKD for a list of unaudited epochs and returns a list of `AuditRequest`s.
    #[instrument(level = "debug", skip_all, fields(namespace = namespace_info.name))]
    async fn poll_for_new_epochs(
        &self,
        namespace_info: &NamespaceInfo,
    ) -> Result<Vec<SerializableAuditBlobName>> {
        let akd = AkdStorageFactory::create_storage(&namespace_info);

        // get the next epoch to audit
        let mut next_epoch = if let Some(last_verified_epoch) = namespace_info.last_verified_epoch {
            last_verified_epoch.next()
        } else {
            namespace_info.starting_epoch
        };

        // Check if the namespace has a proof for the next epoch
        let mut result = Vec::new();
        loop {
            if (result.len()) >= MAX_EPOCHS_PER_POLL {
                // Limit to epochs per poll to avoid overwhelming the system
                info!(
                    namespace = namespace_info.name,
                    "Reached maximum epochs to process in one poll"
                );
                break;
            } else if akd.has_proof(&next_epoch.into()).await {
                debug!(akd = %akd, epoch = %next_epoch, "AKD has published a new proof");

                if let Ok(proof_name) = akd.get_proof_name(&next_epoch.into()).await {
                    // Add the proof name to the queue
                    trace!(akd = %akd, epoch = %next_epoch, proof_name = proof_name.to_string(), "Retrieved proof name");
                    result.push(proof_name.into());
                    // increment the epoch and continue to check for the next one
                    next_epoch = next_epoch.next();
                    continue;
                } else {
                    warn!(akd = %akd, epoch = %next_epoch, "Failed to retrieve proof name for epoch");
                    break;
                }
            } else {
                trace!(akd = %akd, epoch = %next_epoch, "AKD has not published a proof for this epoch, yet");
                break;
            }
        }

        Ok(result)
    }

    /// Downloads the audit proof for the given `AuditRequest`, verifies it, and stores the signature if successful.
    #[instrument(level = "debug", skip_all, fields(namespace = namespace_info.name, blob_name = blob_name.to_string()))]
    async fn process_audit_request(
        &mut self,
        blob_name: &SerializableAuditBlobName,
        namespace_info: &NamespaceInfo,
    ) -> Result<(), AuditError> {
        // Skip epochs before the starting epoch
        if blob_name.epoch < *namespace_info.starting_epoch.value() {
            trace!(
                namespace = namespace_info.name,
                epoch = blob_name.epoch,
                starting_epoch = namespace_info.starting_epoch.value(),
                "Skipping epoch before starting epoch"
            );
            return Ok(());
        }

        // Check if we've already signed this epoch and verify the existing signature if present
        if let Some(_existing_signature) = self.get_and_verify_signature(&blob_name.epoch).await? {
            trace!(
                namespace = namespace_info.name,
                epoch = blob_name.epoch,
                "Existing signature verified, skipping"
            );
            return Ok(());
        }

        // Verify the blob
        let _verified = self.verify_blob(blob_name, namespace_info).await?;

        // sign the proof
        let _signed = self.sign_blob(blob_name, namespace_info).await?;

        Ok(())
    }

    /// Retrieves and verifies an existing signature for the given epoch
    /// Returns Ok(Some(signature)) if found and valid, Ok(None) if not found, or Err on error or if found, but invalid
    async fn get_and_verify_signature(
        &self,
        epoch: &u64,
    ) -> Result<Option<EpochSignature>, AuditError> {
        if let Some(signature) = self.signature_storage.get_signature(&epoch).await? {
            // Verify the signature
            let singing_key_repository = self.signing_key_repository.read().await;
            let verifying_repo = singing_key_repository.verifying_key_repository()?;
            signature.verify(&verifying_repo).await?;

            Ok(Some(signature))
        } else {
            Ok(None)
        }
    }

    async fn verify_blob(
        &self,
        blob_name: &SerializableAuditBlobName,
        namespace_info: &NamespaceInfo,
    ) -> Result<(), AuditError> {
        // download the blob
        let audit_blob = AkdStorageFactory::create_storage(&namespace_info)
            .get_proof(&blob_name.into())
            .await?;
        trace!(
            namespace = namespace_info.name,
            blob_name = blob_name.to_string(),
            "Downloaded audit blob"
        );

        // decode the blob
        let (end_epoch, previous_hash_from_blob, end_hash, proof) = audit_blob
            .decode()
            .map_err(|e| AuditError::LocalAuditorError(e))?;

        // Get and verify the previous epoch's signature to establish the chain
        let previous_hash = if blob_name.epoch == *namespace_info.starting_epoch.value() {
            // For the starting epoch, use the previous hash from the audit blob itself
            // as we trust this to be the initial state
            previous_hash_from_blob
        } else {
            let previous_epoch = blob_name.epoch - 1;

            // Get the previous epoch's signature
            let previous_signature = self
                .get_and_verify_signature(&previous_epoch)
                .await?
                .ok_or_else(|| AuditError::SignatureNotFound(previous_epoch.into()))?;

            trace!(
                namespace = namespace_info.name,
                previous_epoch, "Previous epoch signature verified"
            );

            // Use the hash from the verified previous signature
            previous_signature.epoch_root_hash()?
        };

        // verify the proof using the chained previous hash
        verify_consecutive_append_only(
            &namespace_info.configuration,
            &proof,
            previous_hash,
            end_hash,
            end_epoch,
        )
        .await?;
        trace!(namespace = namespace_info.name, end_epoch, previous_hash = ?previous_hash, end_hash = ?end_hash, "Verified audit proof");
        Ok(())
    }

    async fn sign_blob(
        &mut self,
        blob_name: &SerializableAuditBlobName,
        namespace_info: &NamespaceInfo,
    ) -> Result<(), AuditError> {
        let current_signing_key = self
            .signing_key_repository
            .read()
            .await
            .get_current_signing_key()
            .await?;
        let signature = EpochSignature::sign(
            namespace_info.clone(),
            blob_name.epoch.into(),
            blob_name.current_hash,
            &current_signing_key,
        )?;
        trace!(
            namespace = namespace_info.name,
            blob_name.epoch, "Signed audit proof"
        );

        // store the signature
        self.signature_storage
            .set_signature(&blob_name.epoch, signature)
            .await?;
        trace!(
            namespace = namespace_info.name,
            blob_name.epoch, "Stored signature for audit proof"
        );
        Ok(())
    }
}

async fn interruptible_sleep(duration: Duration, signal: &mut Receiver<()>) -> bool {
    tokio::select! {
        _ = tokio::time::sleep(duration) => {
            // Sleep completed normally
            false
        }
        _ = signal.recv() => {
            // Shutdown signal received
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use akd_watch_common::{
        Epoch, NamespaceStatus,
        akd_configurations::AkdConfiguration,
        storage::test_akd_storage::TestAkdStorage,
        testing::{MockNamespaceRepository, MockSignatureStorage, MockSigningKeyRepository},
    };
    use tokio::sync::broadcast::{self, Receiver, Sender};

    /// Helper to create test namespace
    fn create_test_namespace(name: &str, starting_epoch: u64) -> NamespaceInfo {
        NamespaceInfo {
            name: name.to_string(),
            starting_epoch: Epoch::new(starting_epoch),
            configuration: AkdConfiguration::TestConfiguration,
            log_directory: "test".to_string(),
            last_verified_epoch: Some(Epoch::new(0)),
            status: NamespaceStatus::Online,
        }
    }

    /// Helper to create test components
    fn create_test_components() -> (
        MockNamespaceRepository,
        MockSigningKeyRepository,
        MockSignatureStorage,
        Receiver<()>,
        Sender<()>,
    ) {
        let namespace_repo = MockNamespaceRepository::new();
        let signing_key_repo = MockSigningKeyRepository::new();
        let signature_storage = MockSignatureStorage::new();
        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

        (
            namespace_repo,
            signing_key_repo,
            signature_storage,
            shutdown_rx,
            shutdown_tx,
        )
    }

    #[tokio::test]
    async fn test_interruptible_sleep_completes_normally() {
        let (_shutdown_tx, mut shutdown_rx) = broadcast::channel(1);

        let start = std::time::Instant::now();
        let should_shutdown =
            interruptible_sleep(Duration::from_millis(50), &mut shutdown_rx).await;
        let elapsed = start.elapsed();

        assert!(
            !should_shutdown,
            "Sleep should complete normally without shutdown"
        );
        assert!(
            elapsed >= Duration::from_millis(40),
            "Sleep duration too short: {:?}",
            elapsed
        );
        assert!(
            elapsed <= Duration::from_millis(200),
            "Sleep duration too long: {:?}",
            elapsed
        );
    }

    #[tokio::test]
    async fn test_interruptible_sleep_interrupted_by_shutdown() {
        // Create new shutdown channel for this test
        let (shutdown_tx, mut shutdown_rx) = broadcast::channel(1);

        let sleep_task = tokio::spawn(async move {
            interruptible_sleep(Duration::from_millis(1000), &mut shutdown_rx).await
        });

        // Send shutdown signal immediately
        shutdown_tx.send(()).unwrap();

        let should_shutdown = sleep_task.await.unwrap();
        assert!(should_shutdown, "Shutdown signal should interrupt sleep");
    }

    #[tokio::test]
    async fn test_get_fresh_namespace_info_success() {
        let (mut namespace_repo, signing_key_repo, signature_storage, shutdown_rx, _shutdown_tx) =
            create_test_components();
        let namespace_info = create_test_namespace("test-namespace", 1);
        let mut repo_version = namespace_info.clone();
        repo_version.last_verified_epoch = Some(Epoch::new(100));

        // Add namespace to repository
        namespace_repo.add_namespace(repo_version).await.unwrap();

        let auditor = NamespaceAuditor::new(
            namespace_info.clone(),
            Arc::new(RwLock::new(namespace_repo)),
            Arc::new(RwLock::new(signing_key_repo)),
            signature_storage,
            Duration::from_millis(100),
            shutdown_rx,
        );

        let fresh_info = auditor.get_fresh_namespace_info().await.unwrap();
        assert_eq!(fresh_info.name, "test-namespace");
        assert_eq!(fresh_info.starting_epoch, Epoch::new(1));
        assert_eq!(fresh_info.last_verified_epoch, Some(Epoch::new(100)));
    }

    #[tokio::test]
    async fn test_get_fresh_namespace_info_not_found() {
        let (namespace_repo, signing_key_repo, signature_storage, shutdown_rx, _shutdown_tx) =
            create_test_components();
        let namespace_info = create_test_namespace("test-namespace", 1);

        // Don't add namespace to repository

        let auditor = NamespaceAuditor::new(
            namespace_info,
            Arc::new(RwLock::new(namespace_repo)),
            Arc::new(RwLock::new(signing_key_repo)),
            signature_storage,
            Duration::from_millis(100),
            shutdown_rx,
        );

        let result = auditor.get_fresh_namespace_info().await;
        assert!(result.is_err(), "Should fail when namespace not found");
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_get_and_verify_signature_none_found() {
        let (namespace_repo, signing_key_repo, signature_storage, shutdown_rx, _shutdown_tx) =
            create_test_components();
        let namespace_info = create_test_namespace("test-namespace", 1);

        let auditor = NamespaceAuditor::new(
            namespace_info,
            Arc::new(RwLock::new(namespace_repo)),
            Arc::new(RwLock::new(signing_key_repo)),
            signature_storage,
            Duration::from_millis(100),
            shutdown_rx,
        );

        let result = auditor.get_and_verify_signature(&1).await.unwrap();
        assert!(
            result.is_none(),
            "Should return None when no signature found"
        );
    }

    #[tokio::test]
    async fn test_get_and_verify_signature_found_and_valid() {
        let (namespace_repo, signing_key_repo, mut signature_storage, shutdown_rx, _shutdown_tx) =
            create_test_components();
        let namespace_info = create_test_namespace("test-namespace", 1);

        // Pre-sign epoch 1 using the repository's signing key
        let signing_key = signing_key_repo.get_current_signing_key().await.unwrap();
        let signature = EpochSignature::sign(
            namespace_info.clone(),
            Epoch::new(1),
            [1u8; 32],
            &signing_key,
        )
        .unwrap();
        signature_storage
            .set_signature(&1, signature)
            .await
            .unwrap();

        let auditor = NamespaceAuditor::new(
            namespace_info,
            Arc::new(RwLock::new(namespace_repo)),
            Arc::new(RwLock::new(signing_key_repo)),
            signature_storage,
            Duration::from_millis(100),
            shutdown_rx,
        );

        let result = auditor.get_and_verify_signature(&1).await.unwrap();
        assert!(
            result.is_some(),
            "Should return signature when found and valid"
        );
    }

    // TODO: Test akd polling and processing
    #[tokio::test]
    async fn test_poll_for_new_epochs() {
        let (namespace_repo, signing_key_repo, signature_storage, shutdown_rx, _shutdown_tx) =
            create_test_components();
        let namespace_info = create_test_namespace("test-namespace", 1);

        let auditor = NamespaceAuditor::new(
            namespace_info.clone(),
            Arc::new(RwLock::new(namespace_repo)),
            Arc::new(RwLock::new(signing_key_repo)),
            signature_storage,
            Duration::from_millis(100),
            shutdown_rx,
        );

        let blob_names = auditor.poll_for_new_epochs(&namespace_info).await.unwrap();
        assert_eq!(
            blob_names.len(),
            MAX_EPOCHS_PER_POLL,
            "Should find {} epochs",
            MAX_EPOCHS_PER_POLL
        );
        for i in 1..=MAX_EPOCHS_PER_POLL {
            assert_eq!(
                blob_names[i - 1].previous_hash,
                TestAkdStorage::hash(i as u64),
                "Previous hash for epoch {} should match",
                i
            );
            assert_eq!(
                blob_names[i - 1].current_hash,
                TestAkdStorage::hash(i as u64),
                "Current hash for epoch {} should match",
                i
            );
            assert_eq!(
                blob_names[i - 1].epoch,
                i as u64,
                "Epoch {} should be found",
                i
            );
        }
    }

    #[tokio::test]
    async fn test_verify_blob_blob_not_found() {
        let (namespace_repo, signing_key_repo, signature_storage, shutdown_rx, _shutdown_tx) =
            create_test_components();
        let namespace_info = create_test_namespace("test-namespace", 1);

        // Create a mock blob name
        let blob_name = SerializableAuditBlobName {
            epoch: 200,
            previous_hash: TestAkdStorage::hash(200),
            current_hash: TestAkdStorage::hash(200),
        };

        let auditor = NamespaceAuditor::new(
            namespace_info.clone(),
            Arc::new(RwLock::new(namespace_repo)),
            Arc::new(RwLock::new(signing_key_repo)),
            signature_storage,
            Duration::from_millis(100),
            shutdown_rx,
        );

        // Verify the blob
        let result = auditor.verify_blob(&blob_name, &namespace_info).await;
        assert!(
            result.is_err(),
            "Blob verification should fail for non-existent blob"
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No proof found for blob name")
        );
    }

    #[tokio::test]
    async fn test_verify_blob_previous_signature_not_found() {
        let (namespace_repo, signing_key_repo, signature_storage, shutdown_rx, _shutdown_tx) =
            create_test_components();
        let namespace_info = create_test_namespace("test-namespace", 1);

        // Create a mock blob name
        let blob_name = SerializableAuditBlobName {
            epoch: 2,
            previous_hash: TestAkdStorage::hash(1),
            current_hash: TestAkdStorage::hash(2),
        };

        let auditor = NamespaceAuditor::new(
            namespace_info.clone(),
            Arc::new(RwLock::new(namespace_repo)),
            Arc::new(RwLock::new(signing_key_repo)),
            signature_storage,
            Duration::from_millis(100),
            shutdown_rx,
        );

        // Verify the blob
        let result = auditor.verify_blob(&blob_name, &namespace_info).await;
        assert!(
            result.is_err(),
            "Blob verification should fail when previous signature not found"
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Signature not found for epoch 1"),
        );
    }

    // TODO: verify epoch equal to starting epoch case, but this requires verifiable proof data or service we can mock the verify on
    // TODO: verify blob success case, but this requires verifiable proof data

    #[tokio::test]
    async fn test_sign_blob_success() {
        let (namespace_repo, signing_key_repo, signature_storage, shutdown_rx, _shutdown_tx) =
            create_test_components();
        let namespace_info = create_test_namespace("test-namespace", 1);
        let blob_name = SerializableAuditBlobName {
            epoch: 1,
            previous_hash: TestAkdStorage::hash(1),
            current_hash: TestAkdStorage::hash(1),
        };

        let mut auditor = NamespaceAuditor::new(
            namespace_info.clone(),
            Arc::new(RwLock::new(namespace_repo)),
            Arc::new(RwLock::new(signing_key_repo)),
            signature_storage.clone(),
            Duration::from_millis(100),
            shutdown_rx,
        );
        // Sign the blob
        let result = auditor.sign_blob(&blob_name, &namespace_info).await;
        assert!(result.is_ok(), "Signing blob should succeed");
        // Check that the signature was stored
        let signature = signature_storage
            .get_signature(&blob_name.epoch)
            .await
            .unwrap();
        assert!(
            signature.is_some(),
            "Signature should be stored after signing"
        );
    }

    // TODO: Test failure to sign and set signature, requires mocking for signing and signature storage
    // TODO: test process_audit_request success and failure cases
}
