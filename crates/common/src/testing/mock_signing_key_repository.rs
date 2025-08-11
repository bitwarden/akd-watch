use std::sync::{Arc, RwLock};
use chrono::Duration;
use uuid::Uuid;
use crate::{
    crypto::{SigningKey, VerifyingKey},
    storage::signing_key_repository::{SigningKeyRepository, VerifyingKeyRepository},
};

/// Mock signing key repository for testing
#[derive(Clone, Debug)]
pub struct MockSigningKeyRepository {
    current_key: Arc<RwLock<SigningKey>>,
    expired_keys: Arc<RwLock<Vec<SigningKey>>>,
    should_fail: Arc<RwLock<bool>>,
    key_lifetime: Duration,
}

impl MockSigningKeyRepository {
    pub fn new() -> Self {
        let key_lifetime = Duration::days(30); // Default 30 days
        Self {
            current_key: Arc::new(RwLock::new(SigningKey::generate(key_lifetime))),
            expired_keys: Arc::new(RwLock::new(Vec::new())),
            should_fail: Arc::new(RwLock::new(false)),
            key_lifetime,
        }
    }

    pub fn new_with_lifetime(lifetime: Duration) -> Self {
        Self {
            current_key: Arc::new(RwLock::new(SigningKey::generate(lifetime))),
            expired_keys: Arc::new(RwLock::new(Vec::new())),
            should_fail: Arc::new(RwLock::new(false)),
            key_lifetime: lifetime,
        }
    }

    /// Set a specific signing key for testing
    pub fn set_current_key(&mut self, key: SigningKey) {
        *self.current_key.write().unwrap() = key;
    }

    /// Add an expired key for testing
    pub fn add_expired_key(&mut self, key: SigningKey) {
        self.expired_keys.write().unwrap().push(key);
    }

    /// Set whether operations should fail
    pub fn set_should_fail(&mut self, should_fail: bool) {
        *self.should_fail.write().unwrap() = should_fail;
    }

    /// Get the number of expired keys
    pub fn expired_key_count(&self) -> usize {
        self.expired_keys.read().unwrap().len()
    }

    /// Get the current key ID for testing
    pub fn current_key_id(&self) -> Uuid {
        self.current_key.read().unwrap().key_id()
    }

    /// Force the current key to be expired for testing
    pub fn expire_current_key(&mut self) {
        let mut current = self.current_key.write().unwrap();
        current.expire();
    }
}

impl SigningKeyRepository for MockSigningKeyRepository {
    fn get_current_signing_key(&self) -> impl std::future::Future<Output = SigningKey> + Send {
        let current_key = self.current_key.clone();
        let expired_keys = self.expired_keys.clone();
        let key_lifetime = self.key_lifetime;
        let should_fail = *self.should_fail.read().unwrap();
        
        async move {
            if should_fail {
                // In a real implementation, this might return an error
                // For testing, we'll just return the current key anyway
            }

            let mut current_key_guard = current_key.write().unwrap();
            
            // Check if the current key is expired
            if current_key_guard.is_expired() {
                // Move expired key to expired list
                let expired_key = std::mem::replace(&mut *current_key_guard, SigningKey::generate(key_lifetime));
                expired_keys.write().unwrap().push(expired_key);
            }

            current_key_guard.clone()
        }
    }

    fn force_key_rotation(&self) -> impl std::future::Future<Output = Result<(), String>> + Send {
        let current_key = self.current_key.clone();
        let expired_keys = self.expired_keys.clone();
        let key_lifetime = self.key_lifetime;
        let should_fail = *self.should_fail.read().unwrap();
        
        async move {
            if should_fail {
                return Err("Mock failure for key rotation".to_string());
            }

            let mut current_key_guard = current_key.write().unwrap();
            
            // Expire the current key and move it to expired list
            let mut expired_key = std::mem::replace(&mut *current_key_guard, SigningKey::generate(key_lifetime));
            expired_key.expire();
            expired_keys.write().unwrap().push(expired_key);

            Ok(())
        }
    }

    fn verifying_key_repository(&self) -> impl VerifyingKeyRepository {
        MockVerifyingKeyRepository::new(
            self.current_key.clone(),
            self.expired_keys.clone(),
            self.should_fail.clone(),
        )
    }
}

/// Mock verifying key repository for testing
#[derive(Clone, Debug)]
pub struct MockVerifyingKeyRepository {
    current_key: Arc<RwLock<SigningKey>>,
    expired_keys: Arc<RwLock<Vec<SigningKey>>>,
    should_fail: Arc<RwLock<bool>>,
}

impl MockVerifyingKeyRepository {
    fn new(
        current_key: Arc<RwLock<SigningKey>>,
        expired_keys: Arc<RwLock<Vec<SigningKey>>>,
        should_fail: Arc<RwLock<bool>>,
    ) -> Self {
        Self {
            current_key,
            expired_keys,
            should_fail,
        }
    }
}

impl VerifyingKeyRepository for MockVerifyingKeyRepository {
    fn get_verifying_key(&self, key_id: Uuid) -> impl std::future::Future<Output = Option<VerifyingKey>> + Send {
        let should_fail = *self.should_fail.read().unwrap();
        let current_key = self.current_key.clone();
        let expired_keys = self.expired_keys.clone();
        
        async move {
            if should_fail {
                return None;
            }

            // Check current key
            if let Ok(current_key) = current_key.read().unwrap().verifying_key() {
                if current_key.key_id == key_id {
                    return Some(current_key);
                }
            }

            // Check expired keys
            for expired_key in expired_keys.read().unwrap().iter() {
                if let Ok(verifying_key) = expired_key.verifying_key() {
                    if verifying_key.key_id == key_id {
                        return Some(verifying_key);
                    }
                }
            }

            None
        }
    }
}
