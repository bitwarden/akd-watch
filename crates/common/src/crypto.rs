use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct SigningKey {
    signing_key: Arc<RwLock<ed25519_dalek::SigningKey>>,
    key_id: Uuid,
    created_at: DateTime<Utc>,
    not_after_date: DateTime<Utc>,
}

impl SigningKey {
    pub fn key_id(&self) -> Uuid {
        self.key_id
    }

    pub fn signing_key(&self) -> Arc<RwLock<ed25519_dalek::SigningKey>> {
        Arc::clone(&self.signing_key)
    }

    pub fn new(
        signing_key: ed25519_dalek::SigningKey,
        key_id: Uuid,
        created_at: DateTime<Utc>,
        not_after_date: DateTime<Utc>,
    ) -> Self {
        Self {
            signing_key: Arc::new(RwLock::new(signing_key)),
            key_id,
            created_at,
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
            created_at: Utc::now(),
            not_after_date: Utc::now() + lifetime,
        }
    }
    pub fn verifying_key(&self) -> Result<VerifyingKey, String> {
        Ok(VerifyingKey {
            verifying_key: self
                .signing_key
                .read()
                .map_err(|_| "Poisoned Signing Key Cache")?
                .verifying_key(),
            key_id: self.key_id,
            not_before: self.created_at,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableSigningKey {
    pub signing_key: ed25519_dalek::SigningKey,
    pub key_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub not_after_date: DateTime<Utc>,
}

impl From<SerializableSigningKey> for SigningKey {
    fn from(value: SerializableSigningKey) -> Self {
        SigningKey {
            signing_key: Arc::new(RwLock::new(value.signing_key)),
            key_id: value.key_id,
            created_at: value.created_at,
            not_after_date: value.not_after_date,
        }
    }
}

impl From<SigningKey> for SerializableSigningKey {
    fn from(value: SigningKey) -> Self {
        SerializableSigningKey {
            signing_key: value
                .signing_key
                .read()
                .expect("Poisoned Signing Key Cache")
                .clone(),
            key_id: value.key_id,
            created_at: value.created_at,
            not_after_date: value.not_after_date,
        }
    }
}

impl Serialize for SigningKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let serializable =
            SerializableSigningKey::from(self.clone());
        serializable.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SigningKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let serializable = SerializableSigningKey::deserialize(deserializer)?;
        Ok(SigningKey::from(serializable))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerifyingKey {
    pub verifying_key: ed25519_dalek::VerifyingKey,
    pub key_id: Uuid,
    pub not_before: DateTime<Utc>,
}
