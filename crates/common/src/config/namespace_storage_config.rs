use config::ConfigError;
use serde::{Deserialize, Serialize};

use crate::storage::namespaces::{
    FileNamespaceRepository, InMemoryNamespaceRepository, NamespaceStorage,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum NamespaceStorageConfig {
    #[serde(rename = "InMemory")]
    InMemory,
    #[serde(rename = "File")]
    File,
}

impl NamespaceStorageConfig {
    /// Validate that the namespace storage configuration is complete and usable
    pub fn validate(&self, data_directory: &str) -> Result<(), ConfigError> {
        match self {
            NamespaceStorageConfig::InMemory => Ok(()),
            NamespaceStorageConfig::File => {
                if data_directory.is_empty() {
                    return Err(ConfigError::Message(
                        "Data directory cannot be empty".to_string(),
                    ));
                }

                // Validate the directory exists
                if !std::path::Path::new(data_directory).exists() {
                    return Err(ConfigError::Message(format!(
                        "Data directory does not exist: {}",
                        data_directory
                    )));
                }

                Ok(())
            }
        }
    }

    /// Creates a namespace storage instance based on the given configuration.
    pub fn build_namespace_storage(&self, data_directory: &str) -> NamespaceStorage {
        match self {
            NamespaceStorageConfig::File => {
                NamespaceStorage::File(FileNamespaceRepository::new(data_directory))
            }
            NamespaceStorageConfig::InMemory => {
                NamespaceStorage::InMemory(InMemoryNamespaceRepository::new())
            }
        }
    }
}
