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
    File {
        /// file path where namespace state will be stored
        state_file: String,
    },
}

impl NamespaceStorageConfig {
    /// Validate that the namespace storage configuration is complete and usable
    pub fn validate(&self) -> Result<(), ConfigError> {
        match self {
            NamespaceStorageConfig::InMemory => Ok(()),
            NamespaceStorageConfig::File { state_file } => {
                if state_file.is_empty() {
                    return Err(ConfigError::Message(
                        "Namespace storage state_file cannot be empty".to_string(),
                    ));
                }

                // Validate the directory exists
                let parent = std::path::Path::new(state_file).parent().ok_or_else(|| {
                    ConfigError::Message(
                        "Namespace storage state_file must have a valid parent directory"
                            .to_string(),
                    )
                })?;
                if !parent.exists() {
                    return Err(ConfigError::Message(format!(
                        "Namespace storage state_file parent directory does not exist: {}",
                        parent.display()
                    )));
                }

                Ok(())
            }
        }
    }

    /// Creates a namespace storage instance based on the given configuration.
    pub fn build_namespace_storage(&self) -> NamespaceStorage {
        match self {
            NamespaceStorageConfig::File { state_file } => {
                NamespaceStorage::File(FileNamespaceRepository::new(state_file.clone()))
            }
            NamespaceStorageConfig::InMemory => {
                NamespaceStorage::InMemory(InMemoryNamespaceRepository::new())
            }
        }
    }
}
