use config::ConfigError;
use serde::{Deserialize, Serialize};

use crate::storage::signing_keys::{
    FileSigningKeyRepository, FileVerifyingKeyRepository, SigningKeyStorage, VerifyingKeyStorage,
};

/// Default key lifetime in seconds = 30 days
const DEFAULT_KEY_LIFETIME_SECONDS: i64 = 60 * 60 * 24 * 30; // 30 days

/// Configuration for signing keys
/// If you only need to verify keys, use [`VerifyingConfig`]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SigningConfig {
    /// Key lifetime in seconds
    /// Defaults to 30 days
    #[serde(default = "default_key_lifetime_seconds")]
    pub key_lifetime_seconds: i64,
}

/// Configuration for verifying keys only. This structure is a subset of the signing configuration.
/// If you need to sign data, use [`SigningConfig`].
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VerifyingConfig {}

fn default_key_lifetime_seconds() -> i64 {
    DEFAULT_KEY_LIFETIME_SECONDS
}

impl SigningConfig {
    pub fn validate(&self, data_directory: &str) -> Result<(), ConfigError> {
        validate_directory(data_directory, "Signing key directory")
    }

    /// Panics if initialization of key directory fails
    pub fn build_signing_key_storage(&self, data_directory: &str) -> SigningKeyStorage {
        // For now, we'll only use FileSigningKeyRepository
        // This could be configurable in the future
        SigningKeyStorage::File(FileSigningKeyRepository::new(
            data_directory,
            chrono::Duration::seconds(self.key_lifetime_seconds),
        ))
    }
}

impl VerifyingConfig {
    pub fn validate(&self, data_directory: &str) -> Result<(), ConfigError> {
        validate_directory(data_directory, "Verifying key directory")
    }

    /// Panics if initialization of key directory fails
    pub fn build_verifying_key_storage(
        &self,
        data_directory: &str,
    ) -> Result<VerifyingKeyStorage, ConfigError> {
        let repository = FileVerifyingKeyRepository::new(
            FileSigningKeyRepository::verifying_key_path(data_directory),
        )
        .map_err(|e| {
            ConfigError::Message(format!("Failed to create verifying key storage: {e}"))
        })?;
        Ok(VerifyingKeyStorage::File(repository))
    }
}

fn validate_directory(path: &str, path_name: &str) -> Result<(), ConfigError> {
    if path.is_empty() {
        return Err(ConfigError::Message(
            format!("{path_name} cannot be empty").to_string(),
        ));
    }
    let path = std::path::Path::new(path);
    if !path.exists() {
        return Err(ConfigError::Message(format!(
            "{} does not exist: {}",
            path_name,
            path.display()
        )));
    }
    if !path.is_dir() {
        return Err(ConfigError::Message(format!(
            "{} is not a directory: {}",
            path_name,
            path.display()
        )));
    }
    Ok(())
}
