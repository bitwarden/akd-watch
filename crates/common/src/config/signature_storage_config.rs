use std::collections::HashMap;

use config::ConfigError;
use serde::{Deserialize, Serialize};

use crate::storage::{
    namespaces::{NamespaceRepository, NamespaceStorage},
    signatures::{FilesystemSignatureStorage, InMemorySignatureStorage, SignatureStorage},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum SignatureStorageConfig {
    #[serde(rename = "InMemory")]
    InMemory,

    #[serde(rename = "File")]
    File,

    #[serde(rename = "Azure")]
    Azure {
        /// Azure storage account name
        account_name: String,
        /// Azure container name
        container_name: String,
        /// Azure connection string (required)
        connection_string: Option<String>,
    },
}

impl SignatureStorageConfig {
    /// Validate that the storage configuration is complete and usable
    pub fn validate(&self, data_directory: &str) -> Result<(), ConfigError> {
        match self {
            SignatureStorageConfig::InMemory => Ok(()),
            SignatureStorageConfig::File => {
                if data_directory.is_empty() {
                    return Err(ConfigError::Message(
                        "Data directory cannot be empty".to_string(),
                    ));
                }

                // Check if directory exists
                if !std::path::Path::new(data_directory).exists() {
                    return Err(ConfigError::Message(format!(
                        "Data directory does not exist: {data_directory}"
                    )));
                }

                Ok(())
            }
            SignatureStorageConfig::Azure {
                connection_string, ..
            } => {
                if connection_string.is_none() {
                    Err(ConfigError::Message(
                        "Azure storage requires connection_string in config".to_string(),
                    ))
                } else {
                    Ok(())
                }
            }
        }
    }

    pub fn signatures_directory(data_directory: &str) -> String {
        format!("{data_directory}/signatures")
    }

    pub async fn build_signature_storage(
        &self,
        namespace_storage: &NamespaceStorage,
        data_directory: &str,
    ) -> Result<HashMap<String, SignatureStorage>, ConfigError> {
        let mut storage_map = HashMap::new();

        let namespaces = namespace_storage
            .list_namespaces()
            .await
            .map_err(|e| ConfigError::Message(format!("Failed to list namespaces: {e}")))?;

        match self {
            SignatureStorageConfig::File => {
                for ns_config in namespaces {
                    let ns_directory = format!(
                        "{}/{}",
                        Self::signatures_directory(data_directory),
                        ns_config.name.clone()
                    );
                    storage_map.insert(
                        ns_config.name.clone(),
                        SignatureStorage::Filesystem(FilesystemSignatureStorage::new(
                            &ns_directory,
                        )),
                    );
                }
            }
            SignatureStorageConfig::InMemory => {
                for ns_config in namespaces {
                    storage_map.insert(
                        ns_config.name.clone(),
                        SignatureStorage::InMemory(InMemorySignatureStorage::new()),
                    );
                }
            }
            SignatureStorageConfig::Azure { .. } => {
                todo!("Azure storage not yet implemented for signature storage");
            }
        }

        Ok(storage_map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_config_validation() {
        // Test InMemory - should always be valid
        let inmemory = SignatureStorageConfig::InMemory;
        assert!(inmemory.validate("data_dir").is_ok());

        // Test File with valid directory that exists
        let file_valid = SignatureStorageConfig::File;
        let directory = "/tmp"; // /tmp 
        assert!(file_valid.validate(directory).is_ok());

        // Test File with empty directory
        let file_empty = SignatureStorageConfig::File;
        let directory = "";
        let result = file_empty.validate(directory);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));

        // Test File with non-existent directory
        let file_nonexistent = SignatureStorageConfig::File;
        let directory = "/this/directory/should/not/exist/hopefully/12345";
        let result = file_nonexistent.validate(directory);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));

        // Test Azure with connection string in config
        let azure_with_conn = SignatureStorageConfig::Azure {
            account_name: "test".to_string(),
            container_name: "test".to_string(),
            connection_string: Some("DefaultEndpointsProtocol=https;AccountName=test;".to_string()),
        };
        let directory = "this/shouldn't/matter";
        assert!(azure_with_conn.validate(directory).is_ok());

        // Test Azure without connection string (should fail)
        let azure_no_conn = SignatureStorageConfig::Azure {
            account_name: "test".to_string(),
            container_name: "test".to_string(),
            connection_string: None,
        };
        let result = azure_no_conn.validate(directory);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("requires connection_string")
        );
    }
}
