use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRequest {
    pub namespace: String,
    pub blob_name: String,
    pub epoch: u64,
}
