use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use std::sync::{Arc, RwLock};

use uuid::Uuid;

use crate::AkdWatchError;

#[derive(Clone, Debug)]
pub struct SigningKey {
    signing_key: Arc<RwLock<ed25519_dalek::SigningKey>>,
    key_id: Uuid,
    not_after_date: DateTime<Utc>,
}

impl SigningKey {
    pub fn key_id(&self) -> Uuid {
        self.key_id
    }

    pub fn signing_key(&self) -> Arc<RwLock<ed25519_dalek::SigningKey>> {
        Arc::clone(&self.signing_key)
    }

    pub fn new(signing_key: ed25519_dalek::SigningKey, key_id: Uuid, not_after_date: DateTime<Utc>) -> Self {
        Self {
            signing_key: Arc::new(RwLock::new(signing_key)),
            key_id,
            not_after_date,
        }
    }

    pub fn generate(lifetime: Duration) -> Self {
        let mut secret_key = [0u8; 32];
        rand::rng().fill(&mut secret_key);
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&secret_key);
        let key_id = Uuid::new_v4();
        Self {
            signing_key: Arc::new(RwLock::new(signing_key)),
            key_id,
            not_after_date: Utc::now() + lifetime,
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
            not_after_date: self.not_after_date,
        })
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.not_after_date
    }

    /// Marks this key as expired by setting its expiration date to now
    pub fn expire(&mut self) {
        self.not_after_date = Utc::now();
    }
}

#[derive(Clone, Debug)]
pub struct VerifyingKey {
    pub verifying_key: ed25519_dalek::VerifyingKey,
    pub key_id: Uuid,
    pub not_after_date: DateTime<Utc>,
}
