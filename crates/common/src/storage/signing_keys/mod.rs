mod in_memory_signing_key_repository;

pub use in_memory_signing_key_repository::InMemorySigningKeyRepository;

use std::fmt::Debug;
use uuid::Uuid;

use crate::crypto::{SigningKey, VerifyingKey};

pub trait SigningKeyRepository: Clone + Debug + Send + Sync {
    /// Retrieves the current signing key. If the latest key is expired, it will rotate to the next key and persist the new key.
    fn get_current_signing_key(&self) -> impl Future<Output = SigningKey> + Send;
    /// Updates the current signing key's not_after_date to the current time, generates a new key, and persists it.
    fn force_key_rotation(&self) -> impl Future<Output = Result<(), String>> + Send;
    /// Retrieves a `VerifyingKeyRepository` corresponding to this `SigningKeyRepository`.
    fn verifying_key_repository(&self) -> impl VerifyingKeyRepository;
}

pub trait VerifyingKeyRepository: Clone + Debug + Send + Sync {
    fn get_verifying_key(&self, key_id: Uuid) -> impl Future<Output = Option<VerifyingKey>> + Send;
}
