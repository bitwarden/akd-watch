pub mod namespace_repository;
pub mod signatures;
pub mod signing_keys;
#[cfg(any(test, feature = "testing"))]
pub mod test_akd_storage;
pub mod whatsapp_akd_storage;

use std::{
    fmt::{Debug, Display},
    future::Future,
};

use akd::local_auditing::{AuditBlob, AuditBlobName};

pub trait AkdStorage: Clone + Display + Debug + Send + Sync {
    fn has_proof(&self, epoch: &u64) -> impl Future<Output = bool> + Send;
    fn get_proof_name(
        &self,
        epoch: &u64,
    ) -> impl Future<Output = Result<AuditBlobName, AkdStorageError>> + Send;
    fn get_proof(
        &self,
        name: &AuditBlobName,
    ) -> impl Future<Output = Result<AuditBlob, AkdStorageError>> + Send;
}

// Error for akd proof retrieval
#[derive(Debug, thiserror::Error)]
pub enum AkdStorageError {
    #[error("AKD error: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Custom error: {0}")]
    Custom(String),
}
