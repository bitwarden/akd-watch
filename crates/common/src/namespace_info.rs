use serde::{Deserialize, Serialize};

use crate::{akd_configurations::AkdConfiguration, Epoch};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum NamespaceStatus {
    /// Indicates that the namespace is auditing proofs and has not failed to verify any of them.
    Online,
    Initialization,
    Disabled,
    /// Indicates that a previously audited signature could not be found in signature storage. The Directory must be re-audited from the beginning.
    SignatureLost,
    /// Indicates that the auditor has downloaded a proof that failed verification. Future audits are not performed and the AKD should not be trusted.
    SignatureVerificationFailed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NamespaceInfo {
    pub configuration: AkdConfiguration,
    pub name: String,
    pub log_directory: String,
    pub last_verified_epoch: Option<Epoch>,
    pub starting_epoch: Epoch,
    pub status: NamespaceStatus,
}

impl NamespaceInfo {
    pub fn update_last_verified_epoch(&self, epoch: Epoch) -> Self {
        NamespaceInfo {
            last_verified_epoch: Some(epoch),
            ..self.clone()
        }
    }

    pub fn update_status(&self, status: NamespaceStatus) -> Self {
        NamespaceInfo {
            status,
            ..self.clone()
        }
    }
}
