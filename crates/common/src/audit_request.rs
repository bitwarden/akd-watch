use serde::{Deserialize, Serialize};

use crate::{AkdWatchError, NamespaceInfo};

use akd::local_auditing::AuditBlobName;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRequest {
    pub namespace: NamespaceInfo,
    pub blob_name: String,
}


impl AuditRequest {
    pub fn parse_blob_name(&self) -> Result<AuditBlobName, AkdWatchError> {
        Ok(AuditBlobName::try_from(self.blob_name.as_str())?)
    }
}
