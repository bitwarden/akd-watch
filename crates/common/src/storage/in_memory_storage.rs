use std::{collections::HashMap, sync::{Arc, RwLock}};

use crate::{epoch_signature::EpochSignature, storage::SignatureStorage};

#[derive(Clone, Debug)]
pub struct InMemoryStorage {
    signatures: Arc<RwLock<HashMap<u64, EpochSignature>>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        InMemoryStorage {
            signatures: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl SignatureStorage for InMemoryStorage {
    async fn has_signature(&self, epoch: &u64) -> bool {
        let signatures = self.signatures.read().unwrap();
        signatures.contains_key(epoch)
    }
    async fn get_signature(&self, epoch: &u64) -> Option<EpochSignature> {
        self.signatures.read().unwrap().get(epoch).cloned()
    }

    async fn set_signature(&mut self, epoch: u64, signature: EpochSignature) {
        self.signatures.write().unwrap().insert(epoch, signature);
    }

    async fn latest_signed_epoch(&self) -> u64 {
        self.signatures.read().expect("poisoned signatures lock")
            .keys()
            .cloned()
            .max()
            .unwrap_or(0)
    }
}
