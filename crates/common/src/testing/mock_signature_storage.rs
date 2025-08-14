use crate::{
    EpochSignature,
    storage::signatures::{SignatureRepository, SignatureRepositoryError},
};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

/// Mock signature storage for testing
#[derive(Clone, Debug)]
pub struct MockSignatureStorage {
    signatures: Arc<RwLock<HashMap<u64, EpochSignature>>>,
    should_fail_get: Arc<RwLock<bool>>,
    should_fail_set: Arc<RwLock<bool>>,
}

impl MockSignatureStorage {
    pub fn new() -> Self {
        Self {
            signatures: Arc::new(RwLock::new(HashMap::new())),
            should_fail_get: Arc::new(RwLock::new(false)),
            should_fail_set: Arc::new(RwLock::new(false)),
        }
    }

    /// Add a signature for testing
    pub fn add_test_signature(&mut self, epoch: u64, signature: EpochSignature) {
        self.signatures.write().unwrap().insert(epoch, signature);
    }

    /// Remove a signature for testing
    pub fn remove_test_signature(&mut self, epoch: u64) {
        self.signatures.write().unwrap().remove(&epoch);
    }

    /// Set whether get operations should fail
    pub fn set_should_fail_get(&mut self, should_fail: bool) {
        *self.should_fail_get.write().unwrap() = should_fail;
    }

    /// Set whether set operations should fail
    pub fn set_should_fail_set(&mut self, should_fail: bool) {
        *self.should_fail_set.write().unwrap() = should_fail;
    }

    /// Get the number of stored signatures
    pub fn signature_count(&self) -> usize {
        self.signatures.read().unwrap().len()
    }

    /// Get all stored epochs
    pub fn get_stored_epochs(&self) -> Vec<u64> {
        self.signatures.read().unwrap().keys().cloned().collect()
    }

    /// Clear all signatures
    pub fn clear(&mut self) {
        self.signatures.write().unwrap().clear();
    }
}

impl SignatureRepository for MockSignatureStorage {
    fn has_signature(
        &self,
        epoch: &u64,
    ) -> impl std::future::Future<Output = Result<bool, SignatureRepositoryError>> + Send {
        let result = if *self.should_fail_get.read().unwrap() {
            // For has_signature, we don't really fail - just return false
            false
        } else {
            self.signatures.read().unwrap().contains_key(epoch)
        };
        async move { Ok(result) }
    }

    fn get_signature(
        &self,
        epoch: &u64,
    ) -> impl std::future::Future<Output = Result<Option<EpochSignature>, SignatureRepositoryError>> + Send
    {
        let result = if *self.should_fail_get.read().unwrap() {
            None
        } else {
            self.signatures.read().unwrap().get(epoch).cloned()
        };
        async move { Ok(result) }
    }

    fn set_signature(
        &mut self,
        epoch: &u64,
        signature: EpochSignature,
    ) -> impl std::future::Future<Output = Result<(), SignatureRepositoryError>> + Send {
        if !*self.should_fail_set.read().unwrap() {
            self.signatures.write().unwrap().insert(*epoch, signature);
        }
        async move { Ok(()) }
    }
}
