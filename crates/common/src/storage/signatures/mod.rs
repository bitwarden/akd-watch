mod filesystem_signature_storage;
mod in_memory_signature_storage;

pub use filesystem_signature_storage::FilesystemSignatureStorage;
pub use in_memory_signature_storage::InMemorySignatureStorage;

use crate::EpochSignature;
use std::{fmt::Debug, future::Future};

pub trait SignatureRepository: Clone + Debug + Send + Sync {
    fn has_signature(
        &self,
        epoch: &u64,
    ) -> impl Future<Output = Result<bool, SignatureRepositoryError>> + Send;
    fn get_signature(
        &self,
        epoch: &u64,
    ) -> impl Future<Output = Result<Option<EpochSignature>, SignatureRepositoryError>> + Send;
    fn set_signature(
        &mut self,
        epoch: &u64,
        signature: EpochSignature,
    ) -> impl Future<Output = Result<(), SignatureRepositoryError>> + Send;
}

#[derive(Debug, thiserror::Error)]
pub enum SignatureRepositoryError {
    #[error("{0}")]
    SignatureStorageFileError(#[from] SignatureStorageFileError),
    #[error("Bincode serialization error: {0}")]
    BincodeError(#[from] bincode::error::EncodeError),
    #[error("Bincode deserialization error: {0}")]
    BincodeDecodeError(#[from] bincode::error::DecodeError),
}

#[derive(Debug, thiserror::Error)]
pub enum SignatureStorageFileError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Enum wrapper to support different signature storage implementations
/// 
/// This enum allows applications to work with different storage backends
/// for epoch signatures (Filesystem, InMemory, or future Azure support) 
/// based on configuration.
#[derive(Clone, Debug)]
pub enum SignatureStorage {
    Filesystem(FilesystemSignatureStorage),
    InMemory(InMemorySignatureStorage),
}

impl SignatureRepository for SignatureStorage {
    async fn has_signature(&self, epoch: &u64) -> Result<bool, SignatureRepositoryError> {
        match self {
            SignatureStorage::Filesystem(storage) => storage.has_signature(epoch).await,
            SignatureStorage::InMemory(storage) => storage.has_signature(epoch).await,
        }
    }

    async fn get_signature(&self, epoch: &u64) -> Result<Option<crate::EpochSignature>, SignatureRepositoryError> {
        match self {
            SignatureStorage::Filesystem(storage) => storage.get_signature(epoch).await,
            SignatureStorage::InMemory(storage) => storage.get_signature(epoch).await,
        }
    }

    async fn set_signature(&mut self, epoch: &u64, signature: crate::EpochSignature) -> Result<(), SignatureRepositoryError> {
        match self {
            SignatureStorage::Filesystem(storage) => storage.set_signature(epoch, signature).await,
            SignatureStorage::InMemory(storage) => storage.set_signature(epoch, signature).await,
        }
    }
}
