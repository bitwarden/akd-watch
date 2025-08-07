use std::{collections::HashMap, fmt::Display, sync::{Arc, RwLock}};

use akd::local_auditing::{AuditBlob, AuditBlobName};

use crate::storage::{AkdStorage, AkdStorageError};

/// Test-only AKD storage implementation that stores proofs in memory.
/// This allows for testing without relying on external services.
#[cfg(test)]
#[derive(Clone)]
pub struct TestAkdStorage {
    proofs: Arc<RwLock<HashMap<u64, AuditBlob>>>,
}

#[cfg(test)]
impl std::fmt::Debug for TestAkdStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestAkdStorage")
            .field("proofs", &format!("{} proof(s) stored", self.proofs.read().unwrap().len()))
            .finish()
    }
}

#[cfg(test)]
impl TestAkdStorage {
    pub fn new() -> Self {
        TestAkdStorage {
            proofs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a proof for testing purposes
    pub fn add_proof(&mut self, epoch: u64, blob: AuditBlob) {
        self.proofs.write().unwrap().insert(epoch, blob);
    }

    /// Create a test proof blob with the given epoch
    pub fn create_test_proof(epoch: u64) -> AuditBlob {
        let test_data = format!("test_proof_data_for_epoch_{}", epoch);
        // Use a format similar to WhatsApp's blob names: epoch/hash1/hash2
        // Hash components must be valid hex strings
        let hash1 = format!("{:064x}", epoch - 1); // previous hash 64-character hex string
        let hash2 = format!("{:064x}", epoch); // this hash
        let name = AuditBlobName::try_from(format!("{}/{}/{}", epoch, hash1, hash2).as_str())
            .expect("Failed to create test blob name");
        
        AuditBlob {
            data: test_data.into_bytes(),
            name,
        }
    }
}

#[cfg(test)]
impl Display for TestAkdStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Test AKD Storage")
    }
}

#[cfg(test)]
impl AkdStorage for TestAkdStorage {
    async fn has_proof(&self, epoch: u64) -> bool {
        self.proofs.read().unwrap().contains_key(&epoch)
    }

    async fn get_proof(&self, name: &AuditBlobName) -> Result<AuditBlob, AkdStorageError> {
        // For test storage, we'll try to parse the epoch from the blob name
        let name_str = name.to_string();
        if let Some(epoch_str) = name_str.split('/').next() {
            if let Ok(epoch) = epoch_str.parse::<u64>() {
                self.proofs
                    .read()
                    .unwrap()
                    .get(&epoch)
                    .cloned()
                    .ok_or_else(|| AkdStorageError::Custom(format!("No proof found for blob name: {}", name.to_string())))
            } else {
                Err(AkdStorageError::Custom(format!("Invalid epoch in blob name: {}", name.to_string())))
            }
        } else {
            Err(AkdStorageError::Custom(format!("Invalid blob name format: {}", name.to_string())))
        }
    }

    async fn get_proof_name(&self, epoch: u64) -> Result<AuditBlobName, AkdStorageError> {
        if self.has_proof(epoch).await {
            let hash1 = format!("{:064x}", epoch - 1 );
            let hash2 = format!("{:064x}", epoch);
            AuditBlobName::try_from(format!("{}/{}/{}", epoch, hash1, hash2).as_str())
                .map_err(|_| AkdStorageError::Custom("Invalid blob name format".to_string()))
        } else {
            Err(AkdStorageError::Custom(format!("No proof found for epoch {}", epoch)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_storage_operations() {
        let mut storage = TestAkdStorage::new();
        let epoch = 123;
        
        // Initially no proof
        assert!(!storage.has_proof(epoch).await);
        
        // Add a test proof
        let test_blob = TestAkdStorage::create_test_proof(epoch);
        storage.add_proof(epoch, test_blob.clone());
        
        // Now should have proof
        assert!(storage.has_proof(epoch).await);
        
        // Should be able to get proof name
        let proof_name = storage.get_proof_name(epoch).await.unwrap();
        let expected = format!("123/{:064x}/{:064x}", 122u64, 123u64);
        assert_eq!(proof_name.to_string(), expected);
        
        // Should be able to get the proof
        let retrieved_blob = storage.get_proof(&proof_name).await.unwrap();
        assert_eq!(retrieved_blob.data, test_blob.data);
        assert_eq!(retrieved_blob.name.to_string(), test_blob.name.to_string());
    }
}
