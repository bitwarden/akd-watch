use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, Mutex},
};

use chrono::Duration;
use uuid::Uuid;

use crate::{
    crypto::{SigningKey, VerifyingKey},
    storage::signing_keys::{
        SigningKeyRepository, SigningKeyRepositoryError, VerifyingKeyRepository,
        VerifyingKeyRepositoryError, VerifyingKeyStorage,
    },
};

#[derive(Clone, Debug)]
pub struct InMemorySigningKeyRepository {
    keys: Arc<Mutex<KeyState>>,
    key_lifetime: Duration,
}

#[derive(Debug)]
struct KeyState {
    current_signing_key: SigningKey,
    expired_keys: Vec<SigningKey>,
}

impl InMemorySigningKeyRepository {
    pub fn new(key_lifetime: Duration) -> Self {
        let initial_key = SigningKey::generate(key_lifetime);
        Self {
            keys: Arc::new(Mutex::new(KeyState {
                current_signing_key: initial_key,
                expired_keys: Vec::new(),
            })),
            key_lifetime,
        }
    }

    #[cfg(test)]
    pub fn get_expired_keys_count(&self) -> usize {
        let key_state = self.keys.lock().unwrap();
        key_state.expired_keys.len()
    }

    #[cfg(test)]
    pub fn get_expired_key_ids(&self) -> Vec<Uuid> {
        let key_state = self.keys.lock().unwrap();
        key_state
            .expired_keys
            .iter()
            .map(|key| key.key_id())
            .collect()
    }
}

impl SigningKeyRepository for InMemorySigningKeyRepository {
    async fn get_current_signing_key(&self) -> Result<SigningKey, SigningKeyRepositoryError> {
        let mut key_state = self
            .keys
            .lock()
            .map_err(|_| SigningKeyRepositoryError::Custom("Poisoned key state".into()))?;

        // Check if the current key is expired
        if key_state.current_signing_key.is_expired() {
            // Current key is expired, move it to expired keys
            let expired_key = std::mem::replace(
                &mut key_state.current_signing_key,
                SigningKey::generate(self.key_lifetime),
            );

            key_state.expired_keys.push(expired_key);
        }

        Ok(key_state.current_signing_key.clone())
    }

    async fn force_key_rotation(&self) -> Result<(), SigningKeyRepositoryError> {
        let mut key_state = self.keys.lock().unwrap();

        // Replace current key with new one and get the old key to expire
        let mut existing_key = std::mem::replace(
            &mut key_state.current_signing_key,
            SigningKey::generate(self.key_lifetime),
        );
        existing_key.expire();

        key_state.expired_keys.push(existing_key);

        Ok(())
    }

    fn verifying_key_repository(
        &self,
    ) -> Result<VerifyingKeyStorage, SigningKeyRepositoryError> {
        let mut verifying_keys = Vec::new();

        let key_state = self.keys.lock().unwrap();

        // Add current key
        if let Ok(verifying_key) = key_state.current_signing_key.verifying_key() {
            verifying_keys.push(verifying_key);
        }

        // Add all expired keys
        for signing_key in &key_state.expired_keys {
            if let Ok(verifying_key) = signing_key.verifying_key() {
                verifying_keys.push(verifying_key);
            }
        }

        Ok(VerifyingKeyStorage::InMemory(InMemoryVerifyingKeyRepository::new(verifying_keys)))
    }
}

#[derive(Clone, Debug)]
pub struct InMemoryVerifyingKeyRepository {
    verifying_keys: Arc<Mutex<HashMap<Uuid, VerifyingKey>>>,
}

impl InMemoryVerifyingKeyRepository {
    pub fn new(verifying_keys: Vec<VerifyingKey>) -> Self {
        let mut key_map = HashMap::new();
        for key in verifying_keys {
            key_map.insert(key.key_id, key);
        }
        Self {
            verifying_keys: Arc::new(Mutex::new(key_map)),
        }
    }
}

impl VerifyingKeyRepository for InMemoryVerifyingKeyRepository {
    async fn get_verifying_key(
        &self,
        key_id: Uuid,
    ) -> Result<Option<VerifyingKey>, VerifyingKeyRepositoryError> {
        let keys = self.verifying_keys.lock().unwrap();
        Ok(keys.get(&key_id).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use std::collections::HashSet;

    const LONG_KEY_LIFETIME: Duration = Duration::seconds(3600);
    const SHORT_KEY_LIFETIME: Duration = Duration::milliseconds(1);

    #[tokio::test]
    async fn test_new_repository_has_initial_key() {
        let repo = InMemorySigningKeyRepository::new(LONG_KEY_LIFETIME);
        let key = repo.get_current_signing_key().await.unwrap();

        // Should have a valid key
        assert!(!key.is_expired());
        // Should start with no expired keys
        assert_eq!(repo.get_expired_keys_count(), 0);
    }

    #[tokio::test]
    async fn test_get_current_signing_key_returns_same_key_when_not_expired() {
        let repo = InMemorySigningKeyRepository::new(LONG_KEY_LIFETIME);

        let key1 = repo.get_current_signing_key().await.unwrap();
        let key2 = repo.get_current_signing_key().await.unwrap();

        // Should return the same key
        assert_eq!(key1.key_id(), key2.key_id());
    }

    #[tokio::test]
    async fn test_get_current_signing_key_rotates_expired_key() {
        // Create repo with very short key lifetime
        let repo = InMemorySigningKeyRepository::new(SHORT_KEY_LIFETIME);

        let key1 = repo.get_current_signing_key().await.unwrap();

        // Wait for key to expire
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        let key2 = repo.get_current_signing_key().await.unwrap();

        // Should get a different key
        assert_ne!(key1.key_id(), key2.key_id());
        assert!(!key2.is_expired());
    }

    #[tokio::test]
    async fn test_expired_key_moved_to_expired_keys() {
        let repo = InMemorySigningKeyRepository::new(SHORT_KEY_LIFETIME);

        let key1 = repo.get_current_signing_key().await.unwrap();
        let key1_id = key1.key_id();

        // Initially should have no expired keys
        assert_eq!(repo.get_expired_keys_count(), 0);

        // Wait for key to expire
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        let _key2 = repo.get_current_signing_key().await.unwrap();

        // Should now have one expired key
        assert_eq!(repo.get_expired_keys_count(), 1);
        let expired_key_ids = repo.get_expired_key_ids();
        assert!(expired_key_ids.contains(&key1_id));
    }

    #[tokio::test]
    async fn test_force_key_rotation() {
        let repo = InMemorySigningKeyRepository::new(LONG_KEY_LIFETIME);

        let key1 = repo.get_current_signing_key().await.unwrap();
        let key1_id = key1.key_id();

        // Initially should have no expired keys
        assert_eq!(repo.get_expired_keys_count(), 0);

        // Force rotation
        let result = repo.force_key_rotation().await;
        assert!(result.is_ok());

        let key2 = repo.get_current_signing_key().await.unwrap();

        // Should have a different key
        assert_ne!(key1_id, key2.key_id());
        assert!(!key2.is_expired());

        // Should now have one expired key
        assert_eq!(repo.get_expired_keys_count(), 1);
        let expired_key_ids = repo.get_expired_key_ids();
        assert!(expired_key_ids.contains(&key1_id));

        // Both keys should be available in verifying key repository
        let verifying_repo = repo.verifying_key_repository().unwrap();
        let old_verifying_key = verifying_repo.get_verifying_key(key1_id).await.unwrap();
        let current_verifying_key = verifying_repo
            .get_verifying_key(key2.key_id())
            .await
            .unwrap();
        assert!(old_verifying_key.is_some());
        assert!(current_verifying_key.is_some());
    }

    #[tokio::test]
    async fn test_multiple_rotations_accumulate_expired_keys() {
        let repo = InMemorySigningKeyRepository::new(LONG_KEY_LIFETIME);

        let mut key_ids = Vec::new();

        // Collect initial key
        let initial_key = repo.get_current_signing_key().await.unwrap();
        key_ids.push(initial_key.key_id());

        // Force multiple rotations
        for _ in 0..3 {
            repo.force_key_rotation().await.unwrap();
            let current_key = repo.get_current_signing_key().await.unwrap();
            key_ids.push(current_key.key_id());
        }

        // Should have 4 unique keys (initial + 3 rotations)
        let unique_keys: HashSet<_> = key_ids.iter().collect();
        assert_eq!(unique_keys.len(), 4);

        // Should have 3 expired keys (all but the current one)
        assert_eq!(repo.get_expired_keys_count(), 3);
        let expired_key_ids = repo.get_expired_key_ids();
        assert_eq!(expired_key_ids.len(), 3);

        // All expired keys should be the first 3 keys
        for &key_id in &key_ids[..3] {
            assert!(
                expired_key_ids.contains(&key_id),
                "Key {} should be in expired keys",
                key_id
            );
        }

        // All keys (current + expired) should be available in verifying key repository
        let verifying_repo = repo.verifying_key_repository().unwrap();
        for &key_id in &key_ids {
            let verifying_key = verifying_repo.get_verifying_key(key_id).await.unwrap();
            assert!(
                verifying_key.is_some(),
                "Key {} should be available for verification",
                key_id
            );
        }
    }

    #[tokio::test]
    async fn test_verifying_key_repository_contains_current_and_expired_keys() {
        let repo = InMemorySigningKeyRepository::new(LONG_KEY_LIFETIME);

        let key1 = repo.get_current_signing_key().await.unwrap();
        let key1_id = key1.key_id();

        repo.force_key_rotation().await.unwrap();

        let key2 = repo.get_current_signing_key().await.unwrap();
        let key2_id = key2.key_id();

        let verifying_repo = repo.verifying_key_repository().unwrap();

        // Both keys should be available for verification
        let verifying_key1 = verifying_repo.get_verifying_key(key1_id).await.unwrap();
        let verifying_key2 = verifying_repo.get_verifying_key(key2_id).await.unwrap();

        assert!(verifying_key1.is_some());
        assert!(verifying_key2.is_some());
        assert_eq!(verifying_key1.unwrap().key_id, key1_id);
        assert_eq!(verifying_key2.unwrap().key_id, key2_id);
    }

    #[tokio::test]
    async fn test_verifying_key_repository_returns_none_for_unknown_key() {
        let repo = InMemorySigningKeyRepository::new(LONG_KEY_LIFETIME);
        let verifying_repo = repo.verifying_key_repository().unwrap();

        let unknown_key_id = uuid::Uuid::new_v4();
        let result = verifying_repo
            .get_verifying_key(unknown_key_id)
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let repo = InMemorySigningKeyRepository::new(LONG_KEY_LIFETIME);
        let repo_clone = repo.clone();

        // Spawn multiple tasks that access the repository concurrently
        let mut handles = Vec::new();

        for _ in 0..10 {
            let repo_clone = repo_clone.clone();
            let handle =
                tokio::spawn(async move { repo_clone.get_current_signing_key().await.unwrap() });
            handles.push(handle);
        }

        // All tasks should complete successfully
        let mut keys = Vec::new();
        for handle in handles {
            let key = handle.await.unwrap();
            keys.push(key);
        }

        // All keys should be the same (no rotation should have occurred)
        let first_key_id = keys[0].key_id();
        for key in &keys {
            assert_eq!(key.key_id(), first_key_id);
        }
    }

    #[tokio::test]
    async fn test_concurrent_rotation() {
        let repo = InMemorySigningKeyRepository::new(LONG_KEY_LIFETIME);

        // Spawn multiple rotation tasks concurrently
        let mut handles = Vec::new();

        for _ in 0..5 {
            let repo_clone = repo.clone();
            let handle = tokio::spawn(async move { repo_clone.force_key_rotation().await });
            handles.push(handle);
        }

        // All rotations should complete successfully
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }

        // Should have accumulated expired keys and repository should still work
        let current_key = repo.get_current_signing_key().await.unwrap();
        assert!(!current_key.is_expired());
    }

    #[tokio::test]
    async fn test_repository_is_cloneable() {
        let repo1 = InMemorySigningKeyRepository::new(LONG_KEY_LIFETIME);
        let repo2 = repo1.clone();

        let key1 = repo1.get_current_signing_key().await.unwrap();
        let key2 = repo2.get_current_signing_key().await.unwrap();

        // Both should return the same key (shared state)
        assert_eq!(key1.key_id(), key2.key_id());
    }

    #[tokio::test]
    async fn test_automatic_rotation_vs_forced_rotation() {
        let repo = InMemorySigningKeyRepository::new(SHORT_KEY_LIFETIME);

        let key1 = repo.get_current_signing_key().await.unwrap();

        // Wait for automatic expiration
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        // This should trigger automatic rotation
        let key2 = repo.get_current_signing_key().await.unwrap();
        assert_ne!(key1.key_id(), key2.key_id());

        // This should trigger forced rotation
        repo.force_key_rotation().await.unwrap();
        let key3 = repo.get_current_signing_key().await.unwrap();
        assert_ne!(key2.key_id(), key3.key_id());

        // All three keys should be different
        assert_ne!(key1.key_id(), key2.key_id());
        assert_ne!(key2.key_id(), key3.key_id());
        assert_ne!(key1.key_id(), key3.key_id());
    }
}
