mod filesystem_signature_storage;
mod in_memory_signature_storage;

pub use filesystem_signature_storage::FilesystemSignatureStorage;
pub use in_memory_signature_storage::InMemorySignatureStorage;

use crate::EpochSignature;
use std::{fmt::Debug, future::Future};

pub trait SignatureStorage: Clone + Debug + Send + Sync {
    fn has_signature(
        &self,
        epoch: &u64,
    ) -> impl Future<Output = Result<bool, SignatureStorageError>> + Send;
    fn get_signature(
        &self,
        epoch: &u64,
    ) -> impl Future<Output = Result<Option<EpochSignature>, SignatureStorageError>> + Send;
    fn set_signature(
        &mut self,
        epoch: &u64,
        signature: EpochSignature,
    ) -> impl Future<Output = Result<(), SignatureStorageError>> + Send;
}

#[derive(Debug, thiserror::Error)]
pub enum SignatureStorageError {
    #[error("{0}")]
    SignatureStorageFileError(#[from] SignatureStorageFileError),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum SignatureStorageFileError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
