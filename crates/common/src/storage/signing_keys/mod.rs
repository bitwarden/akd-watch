mod file_signing_key_repository;
mod in_memory_signing_key_repository;

pub use file_signing_key_repository::FileSigningKeyRepository;
pub use in_memory_signing_key_repository::InMemorySigningKeyRepository;

use std::fmt::Debug;
use uuid::Uuid;

use crate::crypto::{SigningKey, VerifyingKey};

pub trait SigningKeyRepository: Clone + Debug + Send + Sync {
    /// Retrieves the current signing key. If the latest key is expired, it will rotate to the next key and persist the new key.
    fn get_current_signing_key(&self) -> impl Future<Output = Result<SigningKey, SigningKeyRepositoryError>> + Send;
    /// Updates the current signing key's not_after_date to the current time, generates a new key, and persists it.
    fn force_key_rotation(&self) -> impl Future<Output = Result<(), SigningKeyRepositoryError>> + Send;
    /// Retrieves a `VerifyingKeyRepository` corresponding to this `SigningKeyRepository`.
    fn verifying_key_repository(&self) -> Result<impl VerifyingKeyRepository, SigningKeyRepositoryError>;
}

pub trait VerifyingKeyRepository: Clone + Debug + Send + Sync {
    fn get_verifying_key(&self, key_id: Uuid) -> impl Future<Output = Result<Option<VerifyingKey>, VerifyingKeyRepositoryError>> + Send;
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
