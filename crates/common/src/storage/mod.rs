mod in_memory_storage;
mod in_memory_queue;
pub mod whatsapp_akd_storage;

use akd::{local_auditing::{AuditBlob, AuditBlobName}};
pub use in_memory_storage::InMemoryStorage;
pub use in_memory_queue::InMemoryQueue;

use crate::{AuditRequest, EpochSignature};

pub trait SignatureStorage {
    fn get_signature(&self, epoch: &u64) -> impl Future<Output = Option<EpochSignature>> + Send;
    fn set_signature(
        &mut self,
        epoch: u64,
        signature: EpochSignature,
    ) -> impl Future<Output = ()> + Send;
}

pub trait AuditRequestQueue {
    fn enqueue(&mut self, request: AuditRequest) -> impl Future<Output = ()> + Send;
    fn enqueue_n(&mut self, requests: Vec<AuditRequest>) -> impl Future<Output = ()> + Send;
    fn dequeue(&mut self) -> impl Future<Output = Option<AuditRequest>> + Send;
    fn dequeue_n(&mut self, n: usize) -> impl Future<Output = Vec<AuditRequest>> + Send;
}

pub trait AkdStorage {
    fn has_proof(&self, epoch: u64) -> impl Future<Output = bool> + Send;
    fn get_proof(&self, name: &AuditBlobName) -> impl Future<Output = Result<AuditBlob, AkdStorageError>> + Send;
}

// Error for akd proof retrieval
#[derive(Debug, thiserror::Error)]
pub enum AkdStorageError {
    #[error("AKD error: {0}")]
    ReqwestError(#[from] reqwest::Error),
}
