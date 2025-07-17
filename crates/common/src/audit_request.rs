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

impl TryFrom<redis::Msg> for AuditRequest {
    type Error = AkdWatchError;

    fn try_from(msg: redis::Msg) -> Result<Self, Self::Error> {
        let payload: String = msg.get_payload()?;
        let audit_request: AuditRequest = serde_json::from_str(&payload).map_err(AkdWatchError::SerdeJsonError)?;
        Ok(audit_request)
    }
}
