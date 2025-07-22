mod in_memory_storage;
mod in_memory_queue;

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
