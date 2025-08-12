mod in_memory_signature_storage;
mod filesystem_signature_storage;
pub mod namespace_repository;
pub mod whatsapp_akd_storage;
pub mod signing_key_repository;
#[cfg(any(test, feature = "testing"))]
pub mod test_akd_storage;

use std::{fmt::{Debug, Display}, future::Future};

use akd::{local_auditing::{AuditBlob, AuditBlobName}};
pub use in_memory_signature_storage::InMemorySignatureStorage;
pub use filesystem_signature_storage::FilesystemSignatureStorage;

use crate::{EpochSignature};

pub trait SignatureStorage: Clone + Debug + Send + Sync {
    fn has_signature(&self, epoch: &u64) -> impl Future<Output = Result<bool, SignatureStorageError>> + Send;
    fn get_signature(&self, epoch: &u64) -> impl Future<Output = Result<Option<EpochSignature>, SignatureStorageError>> + Send;
    fn set_signature(
        &mut self,
        epoch: &u64,
        signature: EpochSignature,
    ) -> impl Future<Output = Result<(),SignatureStorageError>> + Send;
}

pub trait AkdStorage: Clone + Display + Debug + Send + Sync {
    fn has_proof(&self, epoch: &u64) -> impl Future<Output = bool> + Send;
    fn get_proof_name(&self, epoch: &u64) -> impl Future<Output = Result<AuditBlobName, AkdStorageError>> + Send;
    fn get_proof(&self, name: &AuditBlobName) -> impl Future<Output = Result<AuditBlob, AkdStorageError>> + Send;
}

// Error for akd proof retrieval
#[derive(Debug, thiserror::Error)]
pub enum AkdStorageError {
    #[error("AKD error: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Custom error: {0}")]
    Custom(String),
}

#[derive(Debug, thiserror::Error)]
pub enum SignatureStorageError {
    #[error("Signature storage error: {0}")]
    Custom(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}
