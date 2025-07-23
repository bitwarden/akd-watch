use std::{ collections::VecDeque, sync::{Arc, RwLock}};

use crate::{ storage::AuditRequestQueue, AuditRequest};

#[derive(Clone, Debug)]
pub struct InMemoryQueue {
    queue: Arc<RwLock<VecDeque<AuditRequest>>>,
}

impl InMemoryQueue {
    pub fn new() -> Self {
        InMemoryQueue {
            queue: Arc::new(RwLock::new(VecDeque::new())),
        }
    }
}

impl AuditRequestQueue for InMemoryQueue {
    fn enqueue(&mut self, request: AuditRequest) -> impl Future<Output = ()> + Send {
        let queue = self.queue.clone();
        async move {
            let mut queue = queue.write().unwrap();
            queue.push_back(request);
        }
    }

    fn enqueue_n(&mut self, requests: Vec<AuditRequest>) -> impl Future<Output = ()> + Send {
        let queue = self.queue.clone();
        async move {
            let mut queue = queue.write().unwrap();
            for request in requests {
                queue.push_back(request);
            }
        }
    }

    fn dequeue(&mut self) -> impl Future<Output = Option<AuditRequest>> + Send {
        let queue = self.queue.clone();
        async move {
            let mut queue = queue.write().unwrap();
            queue.pop_front()
        }
    }

    fn dequeue_n(&mut self, n: usize) -> impl Future<Output = Vec<AuditRequest>> + Send {
        let queue = self.queue.clone();
        async move {
            let mut queue = queue.write().expect("Failed to acquire write lock on queue");
            queue.drain(0..n).collect()
        }
    }
}
