use rand::Rng;
use std::sync::{Arc, RwLock};

use uuid::Uuid;

use crate::AkdWatchError;

#[derive(Clone, Debug)]
pub struct SigningKey {
    signing_key: Arc<RwLock<ed25519_dalek::SigningKey>>,
    key_id: Uuid,
}

impl SigningKey {
    pub fn key_id(&self) -> Uuid {
        self.key_id
    }

    pub fn signing_key(&self) -> Arc<RwLock<ed25519_dalek::SigningKey>> {
        Arc::clone(&self.signing_key)
    }

    pub fn new(signing_key: ed25519_dalek::SigningKey, key_id: Uuid) -> Self {
        Self {
            signing_key: Arc::new(RwLock::new(signing_key)),
            key_id,
        }
    }

    pub fn generate() -> Self {
        let mut secret_key = [0u8; 32];
        rand::rng().fill(&mut secret_key);
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&secret_key);
        let key_id = Uuid::new_v4();
        Self {
            signing_key: Arc::new(RwLock::new(signing_key)),
            key_id,
        }
    }
    pub fn verifying_key(&self) -> Result<VerifyingKey, AkdWatchError> {
        Ok(VerifyingKey {
            verifying_key: self
                .signing_key
                .read()
                .map_err(|_| AkdWatchError::PoisonedSigningKey)?
                .verifying_key(),
            key_id: self.key_id,
        })
    }
}

pub struct VerifyingKey {
    pub verifying_key: ed25519_dalek::VerifyingKey,
    pub key_id: Uuid,
}
