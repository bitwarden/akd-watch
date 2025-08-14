mod file_signing_key_repository;
mod in_memory_signing_key_repository;

pub use file_signing_key_repository::FileSigningKeyRepository;
pub use in_memory_signing_key_repository::InMemorySigningKeyRepository;

use std::{fmt::Debug, future::Future};
use uuid::Uuid;

use crate::{crypto::{SigningKey, VerifyingKey}, storage::signing_keys::{file_signing_key_repository::FileVerifyingKeyRepository, in_memory_signing_key_repository::InMemoryVerifyingKeyRepository}};

pub trait SigningKeyRepository: Clone + Debug + Send + Sync {
    /// Retrieves the current signing key. If the latest key is expired, it will rotate to the next key and persist the new key.
    fn get_current_signing_key(
        &self,
    ) -> impl Future<Output = Result<SigningKey, SigningKeyRepositoryError>> + Send;
    /// Updates the current signing key's not_after_date to the current time, generates a new key, and persists it.
    fn force_key_rotation(
        &self,
    ) -> impl Future<Output = Result<(), SigningKeyRepositoryError>> + Send;
    /// Retrieves a `VerifyingKeyRepository` corresponding to this `SigningKeyRepository`.
    fn verifying_key_repository(
        &self,
    ) -> Result<VerifyingKeyStorage, SigningKeyRepositoryError>;
}

pub trait VerifyingKeyRepository: Clone + Debug + Send + Sync {
    fn get_verifying_key(
        &self,
        key_id: Uuid,
    ) -> impl Future<Output = Result<Option<VerifyingKey>, VerifyingKeyRepositoryError>> + Send;
}

#[derive(Debug, thiserror::Error)]
pub enum SigningKeyRepositoryError {
    #[error("Signing key repository error: {0}")]
    Custom(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Issue creating verifying key repository: {0}")]
    VerifyingKeyRepositoryError(#[from] VerifyingKeyRepositoryError),
}

#[derive(Debug, thiserror::Error)]
pub enum VerifyingKeyRepositoryError {
    #[error("Verifying key repository error: {0}")]
    Custom(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// Enum wrapper to support different signing key repository implementations
/// 
/// This enum allows applications to work with different storage backends
/// for signing keys (File-based or InMemory) based on configuration.
#[derive(Clone, Debug)]
pub enum SigningKeyStorage {
    File(FileSigningKeyRepository),
    InMemory(InMemorySigningKeyRepository),
}

impl SigningKeyRepository for SigningKeyStorage {
    async fn get_current_signing_key(&self) -> Result<crate::crypto::SigningKey, SigningKeyRepositoryError> {
        match self {
            SigningKeyStorage::File(repo) => repo.get_current_signing_key().await,
            SigningKeyStorage::InMemory(repo) => repo.get_current_signing_key().await,
        }
    }

    async fn force_key_rotation(&self) -> Result<(), SigningKeyRepositoryError> {
        match self {
            SigningKeyStorage::File(repo) => repo.force_key_rotation().await,
            SigningKeyStorage::InMemory(repo) => repo.force_key_rotation().await,
        }
    }

    fn verifying_key_repository(&self) -> Result<VerifyingKeyStorage, SigningKeyRepositoryError> {
        match self {
            SigningKeyStorage::File(repo) => repo.verifying_key_repository(),
            SigningKeyStorage::InMemory(repo) => repo.verifying_key_repository(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum VerifyingKeyStorage {
    File(FileVerifyingKeyRepository),
    InMemory(InMemoryVerifyingKeyRepository),
    #[cfg(any(test, feature = "testing"))]
    Mock(crate::testing::MockVerifyingKeyRepository),
}

impl VerifyingKeyRepository for VerifyingKeyStorage {
    async fn get_verifying_key(
        &self,
        key_id: Uuid,
    ) -> Result<Option<crate::crypto::VerifyingKey>, VerifyingKeyRepositoryError> {
        match self {
            VerifyingKeyStorage::File(repo) => repo.get_verifying_key(key_id).await,
            VerifyingKeyStorage::InMemory(repo) => repo.get_verifying_key(key_id).await,
            #[cfg(any(test, feature = "testing"))]
            VerifyingKeyStorage::Mock(repo) => repo.get_verifying_key(key_id).await
        }
    }
}
