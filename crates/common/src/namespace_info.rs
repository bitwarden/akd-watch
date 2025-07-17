use serde::{Deserialize, Serialize};

use crate::{configurations::AkdConfiguration, AuditVersion, Epoch};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum NamespaceStatus {
    Online,
    Initialization,
    Disabled,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NamespaceInfo {
    pub configuration: AkdConfiguration,
    pub name: String,
    pub log_directory: Option<String>,
    pub last_verified_epoch: Option<Epoch>,
    pub status: NamespaceStatus,
    pub signature_version: AuditVersion,
    // TODO: do we need to track the cipher suite of the namespace audit proofs?
}
