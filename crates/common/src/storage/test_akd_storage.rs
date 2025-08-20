#![cfg(any(test, feature = "testing"))]

use std::{
    collections::HashMap,
    fmt::Display,
    sync::{Arc, RwLock},
};

use akd::local_auditing::{AuditBlob, AuditBlobName};

use crate::storage::{AkdProofDirectoryError, AkdProofNameError, AkdStorage};

/// Test-only AKD storage implementation that stores proofs in memory.
/// This allows for testing without relying on external services.
///
/// Proofs for epochs 1-100 exist, but do not contain real data, if retrieved with `get_proof`.
/// previous_hash and current_hash is always the epoch number repeated 32 times in hex.
/// the data field is always empty.
#[derive(Clone)]
pub struct TestAkdStorage {
    proofs: Arc<RwLock<HashMap<u64, AuditBlob>>>,
}

impl std::fmt::Debug for TestAkdStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestAkdStorage")
            .field(
                "proofs",
                &format!("{} proof(s) stored", self.proofs.read().unwrap().len()),
            )
            .finish()
    }
}

impl Default for TestAkdStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl TestAkdStorage {
    pub fn new() -> Self {
        TestAkdStorage {
            proofs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn hash(epoch: u64) -> [u8; 32] {
        let epoch: u8 = epoch.try_into().expect("Epoch should fit into u8");
        [epoch; 32]
    }

    pub fn hex(epoch: u64) -> String {
        hex::encode(Self::hash(epoch))
    }
}

impl Display for TestAkdStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Test AKD Storage")
    }
}

impl AkdStorage for TestAkdStorage {
    async fn has_proof(&self, epoch: &u64) -> bool {
        epoch > &0 && epoch <= &100
    }

    async fn get_proof(&self, name: &AuditBlobName) -> Result<AuditBlob, AkdProofDirectoryError> {
        if self.has_proof(&name.epoch).await {
            use akd::SingleAppendOnlyProof;

            Ok(AuditBlob::new(
                Self::hash(name.epoch),
                Self::hash(name.epoch),
                name.epoch,
                &SingleAppendOnlyProof {
                    inserted: vec![],
                    unchanged_nodes: vec![],
                },
            )
            .map_err(|_| {
                AkdProofDirectoryError::Custom("Failed to create empty proof".to_string())
            })?)
        } else {
            Err(AkdProofDirectoryError::Custom(format!(
                "No proof found for blob name: {}",
                name.to_string()
            )))
        }
    }

    async fn get_proof_name(&self, epoch: &u64) -> Result<AuditBlobName, AkdProofNameError> {
        if self.has_proof(epoch).await {
            AuditBlobName::try_from(
                format!("{}/{}/{}", epoch, Self::hex(*epoch), Self::hex(*epoch)).as_str(),
            )
            .map_err(|_| AkdProofNameError::AuditBlobNameParsingError)
        } else {
            Err(AkdProofNameError::ProofNotFound(*epoch))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_has_proof() {
        let storage = TestAkdStorage::new();
        assert!(storage.has_proof(&1).await);
        assert!(!storage.has_proof(&0).await);
        assert!(!storage.has_proof(&101).await);
    }

    #[tokio::test]
    async fn test_get_proof_name() {
        let storage = TestAkdStorage::new();
        let name = storage.get_proof_name(&1).await.unwrap();
        assert_eq!(name.epoch, 1);
        assert_eq!(name.previous_hash, TestAkdStorage::hash(1));
        assert_eq!(name.current_hash, TestAkdStorage::hash(1));
    }

    #[tokio::test]
    async fn test_get_proof() {
        let storage = TestAkdStorage::new();
        let name = storage.get_proof_name(&1).await.unwrap();
        let proof = storage.get_proof(&name).await.unwrap();
        assert_eq!(proof.name.epoch, 1);
        assert_eq!(proof.name.previous_hash, TestAkdStorage::hash(1));
        assert_eq!(proof.name.current_hash, TestAkdStorage::hash(1));
        assert_eq!(proof.data, Vec::<u8>::new());
    }
}
