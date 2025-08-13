use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::{
    epoch_signature::EpochSignature,
    storage::signatures::{SignatureStorage, SignatureStorageError},
};

#[derive(Clone, Debug)]
pub struct InMemorySignatureStorage {
    signatures: Arc<RwLock<HashMap<u64, EpochSignature>>>,
}

impl InMemorySignatureStorage {
    pub fn new() -> Self {
        InMemorySignatureStorage {
            signatures: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl SignatureStorage for InMemorySignatureStorage {
    async fn has_signature(&self, epoch: &u64) -> Result<bool, SignatureStorageError> {
        let signatures = self.signatures.read().expect("Poisoned signature storage");
        Ok(signatures.contains_key(epoch))
    }
    async fn get_signature(
        &self,
        epoch: &u64,
    ) -> Result<Option<EpochSignature>, SignatureStorageError> {
        let result = self
            .signatures
            .read()
            .expect("Poisoned signature storage")
            .get(epoch)
            .cloned();
        Ok(result)
    }

    async fn set_signature(
        &mut self,
        epoch: &u64,
        signature: EpochSignature,
    ) -> Result<(), SignatureStorageError> {
        self.signatures
            .write()
            .expect("Poisoned signature storage")
            .insert(*epoch, signature);
        Ok(())
    }
}
