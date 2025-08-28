use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, Mutex},
};

use chrono::Duration;
use serde::{Deserialize, Serialize};
use tracing::debug;
use uuid::Uuid;

use crate::{
    crypto::{SigningKey, VerifyingKey},
    storage::signing_keys::{
        SigningKeyRepository, SigningKeyRepositoryError, VerifyingKeyRepository,
        VerifyingKeyRepositoryError, VerifyingKeyStorage,
    },
};

#[derive(Clone, Debug)]
pub struct FileSigningKeyRepository {
    directory: String,
    keys: Arc<Mutex<KeyState>>,
    key_lifetime: Duration,
}

#[derive(Debug, Serialize, Deserialize)]
struct KeyState {
    current_signing_key: SigningKey,
    expired_keys: Vec<SigningKey>,
}

impl KeyState {
    fn to_verifying_keys(&self) -> Result<Vec<VerifyingKey>, SigningKeyRepositoryError> {
        let mut result = vec![];
        for key in &self.expired_keys {
            let verifying_key = key
                .verifying_key()
                .map_err(SigningKeyRepositoryError::Custom)?;
            result.push(verifying_key);
        }
        result.push(
            self.current_signing_key
                .verifying_key()
                .map_err(SigningKeyRepositoryError::Custom)?,
        );
        Ok(result)
    }
}

impl FileSigningKeyRepository {
    pub fn key_directory(data_directory: &str) -> String {
        format!("{}/keys", data_directory)
    }

    pub fn new(data_directory: &str, key_lifetime: Duration) -> Self {
        let directory = Self::key_directory(data_directory);

        // Create the directory if it doesn't exist
        std::fs::create_dir_all(&directory)
            .expect("Failed to create signing key directory");

        // Load from file if it exists, otherwise create a new one
        let initial_key_state =
            if std::path::Path::new(&Self::signing_key_path(&directory)).exists() {
                let file_content = std::fs::read_to_string(Self::signing_key_path(&directory))
                    .expect("Failed to read signing key file");
                serde_json::from_str::<KeyState>(&file_content)
                    .expect("Failed to deserialize signing key state")
            } else {
                KeyState {
                    current_signing_key: SigningKey::generate(key_lifetime),
                    expired_keys: Vec::new(),
                }
            };

        let new = Self {
            directory,
            keys: Arc::new(Mutex::new(initial_key_state)),
            key_lifetime,
        };
        new.persist()
            .expect("Failed to persist initial signing key");
        new
    }

    fn _signing_key_path(&self) -> String {
        Self::signing_key_path(&self.directory)
    }

    pub fn signing_key_path(dir: &str) -> String {
        format!("{dir}/keys.json")
    }

    fn _verifying_key_path(&self) -> String {
        Self::verifying_key_path(&self.directory)
    }

    pub fn verifying_key_path(dir: &str) -> String {
        format!("{dir}/keys_verifying.json")
    }

    pub fn rotate_signing_key(&self) -> Result<SigningKey, SigningKeyRepositoryError> {
        debug!("Rotating signing key");
        let mut key_state = self.keys.lock().unwrap();

        // Replace current key with new one and get the old key to expire
        let new_key = SigningKey::generate(self.key_lifetime);
        let mut existing_key =
            std::mem::replace(&mut key_state.current_signing_key, new_key.clone());
        existing_key.expire();

        key_state.expired_keys.push(existing_key);

        // Persist the new signing key to file
        self.persist()?;

        Ok(new_key)
    }

    fn persist(&self) -> Result<(), SigningKeyRepositoryError> {
        // first persist the signing keys
        let path = self._signing_key_path();
        let key_state = self.keys.lock().expect("Mutex poisoned");
        let serialized = serde_json::to_string(&*key_state)?;
        debug!("Persisting signing keys to {}", path);
        std::fs::write(path, serialized).map_err(SigningKeyRepositoryError::IoError)?;

        // then persist the verifying keys
        let verifying_keys = key_state.to_verifying_keys()?;
        let verifying_path = self._verifying_key_path();
        let serialized_verifying = serde_json::to_string(&verifying_keys)?;
        debug!("Persisting verifying keys to {}", verifying_path);
        std::fs::write(verifying_path, serialized_verifying)
            .map_err(SigningKeyRepositoryError::IoError)?;
        Ok(())
    }
}

impl SigningKeyRepository for FileSigningKeyRepository {
    async fn get_current_signing_key(&self) -> Result<SigningKey, SigningKeyRepositoryError> {
        // Check if we need to rotate the signing key
        let should_rotate = {
            let key_state = self.keys.lock().unwrap();
            key_state.current_signing_key.is_expired()
        };

        let current_key = if should_rotate {
            // This locks the keys mutex, so we need to be careful about locks in this context
            self.rotate_signing_key()?
        } else {
            self.keys.lock().unwrap().current_signing_key.clone()
        };

        Ok(current_key)
    }

    async fn force_key_rotation(&self) -> Result<(), SigningKeyRepositoryError> {
        self.rotate_signing_key()?;
        Ok(())
    }

    fn verifying_key_repository(&self) -> Result<VerifyingKeyStorage, SigningKeyRepositoryError> {
        Ok(VerifyingKeyStorage::File(FileVerifyingKeyRepository::new(
            self._verifying_key_path(),
        )?))
    }
}

#[derive(Clone, Debug)]
pub struct FileVerifyingKeyRepository {
    path: String,
    verifying_keys: Arc<Mutex<HashMap<Uuid, VerifyingKey>>>,
}

impl FileVerifyingKeyRepository {
    pub fn new(path: String) -> Result<Self, VerifyingKeyRepositoryError> {
        let new = Self {
            path,
            verifying_keys: Arc::new(Mutex::new(HashMap::new())),
        };
        new.populate_in_memory_map()?;
        Ok(new)
    }

    fn get_verifying_key_from_memory(&self, key_id: Uuid) -> Option<VerifyingKey> {
        let keys = self.verifying_keys.lock().expect("Mutex poisoned");
        keys.get(&key_id).cloned()
    }

    fn populate_in_memory_map(&self) -> Result<(), VerifyingKeyRepositoryError> {
        let mut keys = self.verifying_keys.lock().expect("Mutex poisoned");
        if !std::path::Path::new(&self.path).exists() {
            // No file, so nothing to populate
            return Ok(());
        }

        let file_content =
            std::fs::read_to_string(&self.path).map_err(VerifyingKeyRepositoryError::IoError)?;
        let verifying_keys: Vec<VerifyingKey> = serde_json::from_str(&file_content)
            .map_err(VerifyingKeyRepositoryError::SerializationError)?;
        for key in verifying_keys {
            keys.insert(key.key_id, key);
        }
        Ok(())
    }
}

impl VerifyingKeyRepository for FileVerifyingKeyRepository {
    async fn get_verifying_key(
        &self,
        key_id: Uuid,
    ) -> Result<Option<VerifyingKey>, VerifyingKeyRepositoryError> {
        // If the key is in the map, return it
        if let Some(key) = self.get_verifying_key_from_memory(key_id) {
            return Ok(Some(key));
        }

        // re-populate keys and return the new key
        self.populate_in_memory_map()?;
        Ok(self.get_verifying_key_from_memory(key_id))
    }

    async fn list_keys(&self) -> Result<Vec<VerifyingKey>, VerifyingKeyRepositoryError> {
        // Always populate the in-memory map before listing keys
        self.populate_in_memory_map()?;
        // Return all keys in the map
        let keys = self.verifying_keys.lock().expect("Mutex poisoned");
        Ok(keys.values().cloned().collect())
    }
}
