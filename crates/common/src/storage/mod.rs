mod in_memory_storage;
pub mod namespace_repository;
pub mod whatsapp_akd_storage;

use std::{fmt::{Debug, Display}};

use akd::{local_auditing::{AuditBlob, AuditBlobName}};
use async_trait::async_trait;
pub use in_memory_storage::InMemoryStorage;

use crate::{EpochSignature};

#[async_trait]
pub trait SignatureStorage: Clone + Debug + Send + Sync {
    async fn has_signature(&self, epoch: &u64) -> bool;
    async fn get_signature(&self, epoch: &u64) -> Option<EpochSignature>;
    async fn set_signature(
        &mut self,
        epoch: u64,
        signature: EpochSignature,
    ) -> ();
    async fn latest_signed_epoch(&self) -> u64;
}

pub trait AkdStorage: Clone + Display + Debug + Send + Sync {
    fn has_proof(&self, epoch: u64) -> impl Future<Output = bool> + Send;
    fn get_proof_name(&self, epoch: u64) -> impl Future<Output = Result<AuditBlobName, AkdStorageError>> + Send;
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
