use serde::{Deserialize, Serialize};

use akd::local_auditing::AuditBlobName;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableAuditBlobName {
    pub epoch: u64,
    pub previous_hash: akd::Digest,
    pub current_hash: akd::Digest,
}

impl From<AuditBlobName> for SerializableAuditBlobName {
    fn from(blob_name: AuditBlobName) -> Self {
        SerializableAuditBlobName {
            epoch: blob_name.epoch,
            previous_hash: blob_name.previous_hash,
            current_hash: blob_name.current_hash,
        }
    }
}

impl From<&AuditBlobName> for SerializableAuditBlobName {
    fn from(blob_name: &AuditBlobName) -> Self {
        SerializableAuditBlobName {
            epoch: blob_name.epoch,
            previous_hash: blob_name.previous_hash,
            current_hash: blob_name.current_hash,
        }
    }
}

impl From<SerializableAuditBlobName> for AuditBlobName {
    fn from(blob_name: SerializableAuditBlobName) -> Self {
        AuditBlobName {
            epoch: blob_name.epoch,
            previous_hash: blob_name.previous_hash,
            current_hash: blob_name.current_hash,
        }
    }
}

impl From<&SerializableAuditBlobName> for AuditBlobName {
    fn from(blob_name: &SerializableAuditBlobName) -> Self {
        AuditBlobName {
            epoch: blob_name.epoch,
            previous_hash: blob_name.previous_hash,
            current_hash: blob_name.current_hash,
        }
    }
}

impl std::fmt::Display for SerializableAuditBlobName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let audit_blob_name: AuditBlobName = self.clone().into();
        write!(f, "{}", audit_blob_name.to_string())
    }
}
