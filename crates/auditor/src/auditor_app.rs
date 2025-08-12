use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use akd_watch_common::storage::{
    namespace_repository::{InMemoryNamespaceRepository, NamespaceRepository},
    signatures::{FilesystemSignatureStorage, SignatureStorage},
    signing_keys::{InMemorySigningKeyRepository, SigningKeyRepository},
};
use anyhow::{Context, Result};
use futures_util::future;
use tokio::sync::broadcast;
use tracing::{info, warn};

use crate::config::{AuditorConfig, StorageConfig};
use crate::namespace_auditor::NamespaceAuditor;

/// Main auditor application
pub struct AuditorApp<NR, SKR, SS> {
    namespace_repository: Arc<RwLock<NR>>,
    signing_key_repository: Arc<RwLock<SKR>>,
    signature_storage_map: HashMap<String, SS>,
    sleep_duration: Duration,
    shutdown_tx: broadcast::Sender<()>,
}

impl
    AuditorApp<
        InMemoryNamespaceRepository,
        InMemorySigningKeyRepository,
        FilesystemSignatureStorage,
    >
{
    /// Build the auditor application from configuration
    pub async fn from_config(config: AuditorConfig) -> Result<Self> {
        info!(
            "Initializing auditor with {} namespaces",
            config.namespaces.len()
        );

        // Initialize repositories and storage based on config
        let namespace_repository = Self::init_namespace_repository(&config).await?;
        let signature_storage_map = Self::init_signature_storage(&config).await?;
        let signing_key_repository =
            Arc::new(RwLock::new(Self::init_signing_key_repository(&config)));

        // Create shutdown channel
        let (shutdown_tx, _) = broadcast::channel(1);

        Ok(AuditorApp {
            namespace_repository: Arc::new(RwLock::new(namespace_repository)),
            signing_key_repository,
            signature_storage_map,
            sleep_duration: config.sleep_duration(),
            shutdown_tx,
        })
    }
}

impl<NR, SKR, SS> AuditorApp<NR, SKR, SS>
where
    NR: NamespaceRepository + Send + Sync + 'static,
    SKR: SigningKeyRepository + Send + Sync + 'static,
    SS: SignatureStorage + Send + Sync + Clone + 'static,
{
    /// Run the auditor application
    pub async fn run(&self) -> Result<()> {
        // Get all namespaces from the repository
        let namespace_infos = self
            .namespace_repository
            .read()
            .await
            .list_namespaces()
            .await
            .with_context(|| "Failed to get namespaces from repository")?;

        let mut handles = Vec::new();

        for namespace_info in namespace_infos {
            let signature_storage = self
                .signature_storage_map
                .get(&namespace_info.name)
                .with_context(|| {
                    format!(
                        "Missing signature storage for namespace {}",
                        namespace_info.name
                    )
                })?
                .clone();

            let auditor = NamespaceAuditor::new(
                namespace_info.clone(),
                self.namespace_repository.clone(),
                self.signing_key_repository.clone(),
                signature_storage,
                self.sleep_duration,
                self.shutdown_tx.subscribe(),
            );

            let handle = tokio::spawn(async move {
                if let Err(e) = auditor.run().await {
                    warn!(
                        namespace = namespace_info.name,
                        error = %e,
                        "Namespace auditor exited with error"
                    );
                }
            });

            handles.push(handle);
        }

        info!("Started {} namespace auditors", handles.len());

        // Wait for all auditors to complete
        let results = future::join_all(handles).await;
        for result in results {
            if let Err(e) = result {
                warn!(error = %e, "Auditor task completed with error");
            }
        }

        info!("All auditors completed");
        Ok(())
    }

    /// Gracefully shutdown all auditors
    pub fn shutdown(&self) -> Result<()> {
        info!("Initiating graceful shutdown");
        self.shutdown_tx
            .send(())
            .map_err(|_| anyhow::anyhow!("Failed to send shutdown signal - no receivers"))?;
        Ok(())
    }

    // Private initialization methods that can be configured based on config in the future
    async fn init_namespace_repository(
        config: &AuditorConfig,
    ) -> Result<InMemoryNamespaceRepository> {
        let mut namespace_repository = InMemoryNamespaceRepository::new();
        let existing_namespaces = namespace_repository
            .list_namespaces()
            .await
            .unwrap_or_default();

        for ns_config in &config.namespaces {
            let existing_info = existing_namespaces
                .iter()
                .find(|info| info.name == ns_config.name);

            let (namespace_info, status_changed) = ns_config
                .to_namespace_info(existing_info)
                .with_context(|| format!("Configuration error for namespace {}", ns_config.name))?;

            if existing_info.is_none() {
                info!(namespace = ?namespace_info, "Adding new namespace to repository");
                namespace_repository
                    .add_namespace(namespace_info.clone())
                    .await
                    .with_context(|| format!("Failed to add namespace {}", ns_config.name))?;
            } else if status_changed {
                info!(
                    namespace = ns_config.name,
                    old_status = ?existing_info.unwrap().status,
                    new_status = ?namespace_info.status,
                    "Updating namespace status in repository"
                );
                namespace_repository
                    .update_namespace(namespace_info.clone())
                    .await
                    .with_context(|| format!("Failed to update namespace {}", ns_config.name))?;
            } else {
                info!(
                    namespace = ns_config.name,
                    "Using existing namespace from repository (no changes)"
                );
            }
        }

        Ok(namespace_repository)
    }

    async fn init_signature_storage(
        config: &AuditorConfig,
    ) -> Result<HashMap<String, FilesystemSignatureStorage>> {
        let mut storage_map = HashMap::new();

        let storage_directory = match &config.storage {
            StorageConfig::File { directory } => directory.clone(),
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported storage type: {:?}",
                    config.storage
                ));
            }
        };

        for ns_config in &config.namespaces {
            // TODO: Could configure storage type based on config in the future
            storage_map.insert(
                ns_config.name.clone(),
                FilesystemSignatureStorage::new(storage_directory.clone()),
            );
        }

        Ok(storage_map)
    }

    fn init_signing_key_repository(config: &AuditorConfig) -> InMemorySigningKeyRepository {
        // TODO: Could configure repository type based on config in the future
        InMemorySigningKeyRepository::new(chrono::Duration::seconds(
            config.signing.key_lifetime_seconds,
        ))
    }
}
