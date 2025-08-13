pub mod namespace_repository;
pub mod signatures;
pub mod signing_keys;
#[cfg(any(test, feature = "testing"))]
pub mod test_akd_storage;
pub mod whatsapp_akd_storage;

use std::{
    fmt::{Debug, Display},
    future::Future,
    str::Utf8Error,
};

use akd::local_auditing::{AuditBlob, AuditBlobName};

pub trait AkdStorage: Clone + Display + Debug + Send + Sync {
    fn has_proof(&self, epoch: &u64) -> impl Future<Output = bool> + Send;
    fn get_proof_name(
        &self,
        epoch: &u64,
    ) -> impl Future<Output = Result<AuditBlobName, AkdProofNameError>> + Send;
    fn get_proof(
        &self,
        name: &AuditBlobName,
    ) -> impl Future<Output = Result<AuditBlob, AkdProofDirectoryError>> + Send;
}

// Error for akd proof retrieval
#[derive(Debug, thiserror::Error)]
pub enum AkdProofDirectoryError {
    #[error("AKD error: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Key name parsing error: {0}")]
    KeyNameParsingError(#[from] Utf8Error),
    #[error("XML parsing error: {0}")]
    XmlParsingError(#[from] quick_xml::Error),
    #[error("Custom error: {0}")]
    Custom(String),
}

#[derive(Debug, thiserror::Error)]
pub enum AkdProofError {
    #[error("AKD error: {0}")]
    ReqwestError(#[from] reqwest::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum AkdProofNameError {
    #[error("{0}")]
    AkdProofDirectoryError(#[from] AkdProofDirectoryError),
    #[error("AuditBlobName parsing error")]
    AuditBlobNameParsingError,
    #[error("Proof not found for epoch {0}")]
    ProofNotFound(u64),
}
