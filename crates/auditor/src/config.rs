use config::{Config, ConfigError, File, Environment};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use akd_watch_common::{akd_configurations::AkdConfiguration, NamespaceInfo, NamespaceStatus};

const DEFAULT_SLEEP_SECONDS: u64 = 30; // Default to 30 seconds
const DEFAULT_KEY_LIFETIME_SECONDS: i64 = 60 * 60 * 24 * 30; // Default to 30 days


#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AkdConfigurationType {
    WhatsAppV1,
    BitwardenV1,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ConfigNamespaceStatus {
    Online,
    Disabled,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuditorConfig {
    /// How long to sleep between audit cycles
    #[serde(default = "default_sleep_seconds")]
    pub sleep_seconds: u64,
    
    /// Namespace configurations to audit
    pub namespaces: Vec<NamespaceConfig>,
    
    /// Signing key configuration
    pub signing: SigningConfig,
    
    /// Storage configuration
    pub storage: StorageConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NamespaceConfig {
    /// Name of the namespace
    pub name: String,
    
    /// Configuration type
    pub configuration_type: AkdConfigurationType,
    
    /// Directory where logs are stored
    pub log_directory: String,
    
    /// Starting epoch for auditing (only used if no existing namespace info found)
    #[serde(default)]
    pub starting_epoch: u64,
    
    /// Status
    pub status: ConfigNamespaceStatus,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SigningConfig {
    /// Path to the signing key file
    pub key_file: String,
    #[serde(default = "default_key_lifetime_seconds")]
    pub key_lifetime_seconds: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum StorageConfig {
    #[serde(rename = "InMemory")]
    InMemory,
    
    #[serde(rename = "File")]
    File {
        /// Directory path where files will be stored
        directory: String,
    },
    
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

impl StorageConfig {
    /// Validate that the storage configuration is complete and usable
    pub fn validate(&self) -> Result<(), ConfigError> {
        match self {
            StorageConfig::InMemory => Ok(()),
            StorageConfig::File { directory } => {
                if directory.is_empty() {
                    return Err(ConfigError::Message("File storage directory cannot be empty".to_string()));
                }
                
                // Check if directory exists
                if !std::path::Path::new(directory).exists() {
                    return Err(ConfigError::Message(
                        format!("File storage directory does not exist: {}", directory)
                    ));
                }
                
                Ok(())
            }
            StorageConfig::Azure { connection_string, .. } => {
                if connection_string.is_none() {
                    Err(ConfigError::Message(
                        "Azure storage requires connection_string in config".to_string()
                    ))
                } else {
                    Ok(())
                }
            }
        }
    }
}

impl AuditorConfig {
    /// Load configuration from multiple sources in order of priority:
    /// 1. Configuration file (config.toml, config.yaml, config.json)
    /// 2. Environment variables (prefixed with AUDITOR_)
    pub fn load() -> Result<Self, ConfigError> {
        let config = Config::builder()
            // Start with default config file
            .add_source(File::with_name("config").required(false))
            // Add environment variables with prefix "AUDITOR_"
            .add_source(Environment::with_prefix("AUDITOR").separator("_"))
            .build()?;
        
        let auditor_config: Self = config.try_deserialize()?;
        
        auditor_config.validate()?;
        
        Ok(auditor_config)
    }
    
    /// Load configuration from a specific file
    #[allow(dead_code)]
    pub fn load_from_file(path: &str) -> Result<Self, ConfigError> {
        let config = Config::builder()
            .add_source(File::with_name(path))
            .add_source(Environment::with_prefix("AUDITOR").separator("_"))
            .build()?;
        
        let auditor_config: Self = config.try_deserialize()?;
        
        auditor_config.validate()?;
        
        Ok(auditor_config)
    }
    
    /// Validate the entire auditor configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate storage configuration
        self.storage.validate()?;
        
        // TODO: Add validation for other configuration sections as needed
        // - signing key file existence
        // - namespace log directory validation
        // - namespace name uniqueness
        
        Ok(())
    }
    
    /// Get sleep duration as Duration type
    pub fn sleep_duration(&self) -> Duration {
        Duration::from_secs(self.sleep_seconds)
    }
}

impl NamespaceConfig {
    /// Convert to NamespaceInfo from common crate
    /// If an existing namespace_info is provided, it will preserve the last_verified_epoch
    /// Otherwise, it will use the starting_epoch from config
    /// 
    /// Returns a tuple of (NamespaceInfo, bool) where the bool indicates if the status was changed. New namespaces will not count as a change.
    pub fn to_namespace_info(&self, existing_namespace_info: Option<&NamespaceInfo>) -> Result<(NamespaceInfo, bool), ConfigError> {
        let configuration = match self.configuration_type {
            AkdConfigurationType::WhatsAppV1 => AkdConfiguration::WhatsAppV1Configuration,
            AkdConfigurationType::BitwardenV1 => AkdConfiguration::BitwardenV1Configuration,
        };
        
        // Handle status transitions
        let (status, status_changed) = Self::resolve_status_transition(
            &self.status,
            existing_namespace_info.map(|info| &info.status)
        );
        
        // Use existing last_verified_epoch if available, otherwise use starting_epoch from config
        let last_verified_epoch = existing_namespace_info
            .map(|info| info.last_verified_epoch)
            .unwrap_or_else(|| self.starting_epoch.into());
        
        let namespace_info = NamespaceInfo {
            configuration,
            name: self.name.clone(),
            log_directory: self.log_directory.clone(),
            last_verified_epoch,
            status,
        };
        
        Ok((namespace_info, status_changed))
    }
    
    /// Resolve status transitions based on configuration and existing status
    /// 
    /// Returns (new_status, status_changed)
    fn resolve_status_transition(
        config_status: &ConfigNamespaceStatus,
        existing_status: Option<&NamespaceStatus>
    ) -> (NamespaceStatus, bool) {       
        let desired_status = match config_status {
            ConfigNamespaceStatus::Online => NamespaceStatus::Online,
            ConfigNamespaceStatus::Disabled => NamespaceStatus::Disabled,
        };
        
        match existing_status {
            // New namespace - use config status, no change to report
            None => (desired_status, false),
            // Error states are preserved and never count as changes
            Some(NamespaceStatus::SignatureLost | NamespaceStatus::SignatureVerificationFailed) => {
                (existing_status.unwrap().clone(), false)
            }
            // All other states can transition normally
            Some(current_status) => {
                let changed = *current_status != desired_status;
                (desired_status, changed)
            }
        }
    }
}

// Default values for serde defaults
fn default_sleep_seconds() -> u64 {
    DEFAULT_SLEEP_SECONDS
}

fn default_key_lifetime_seconds() -> i64 {
    DEFAULT_KEY_LIFETIME_SECONDS
}

#[cfg(test)]
mod tests {
    use akd_watch_common::akd_configurations::AkdConfiguration;

    use super::*;
    
    #[test]
    fn test_namespace_config_conversion() {
        let namespace_config = NamespaceConfig {
            name: "test".to_string(),
            configuration_type: AkdConfigurationType::WhatsAppV1,
            log_directory: "logs/test".to_string(),
            starting_epoch: 5,
            status: ConfigNamespaceStatus::Online,
        };
        
        // Test with no existing namespace info (should use starting_epoch)
        let (namespace_info, status_changed) = namespace_config.to_namespace_info(None).unwrap();
        assert_eq!(namespace_info.name, "test");
        assert_eq!(namespace_info.last_verified_epoch, 5u64.into());
        assert!(!status_changed); // New namespace, so no change
        
        // Test with existing namespace info (should preserve existing last_verified_epoch)
        let existing_info = NamespaceInfo {
            configuration: AkdConfiguration::WhatsAppV1Configuration,
            name: "test".to_string(),
            log_directory: "logs/test".to_string(),
            last_verified_epoch: 10u64.into(),
            status: NamespaceStatus::Online,
        };
        
        let (namespace_info, status_changed) = namespace_config.to_namespace_info(Some(&existing_info)).unwrap();
        assert_eq!(namespace_info.name, "test");
        assert_eq!(namespace_info.last_verified_epoch, 10u64.into()); // Should preserve existing value
        assert!(!status_changed); // Both Online, no change
    }
    
    #[test]
    fn test_strong_typing() {
        // Test that AkdConfigurationType enum works correctly
        let whatsapp_config = NamespaceConfig {
            name: "test".to_string(),
            configuration_type: AkdConfigurationType::WhatsAppV1,
            log_directory: "logs/test".to_string(),
            starting_epoch: 0,
            status: ConfigNamespaceStatus::Online,
        };
        
        let (namespace_info, _) = whatsapp_config.to_namespace_info(None).unwrap();
        matches!(namespace_info.configuration, AkdConfiguration::WhatsAppV1Configuration);
        
        let bitwarden_config = NamespaceConfig {
            name: "test".to_string(),
            configuration_type: AkdConfigurationType::BitwardenV1,
            log_directory: "logs/test".to_string(),
            starting_epoch: 0,
            status: ConfigNamespaceStatus::Disabled,
        };
        
        let (namespace_info, _) = bitwarden_config.to_namespace_info(None).unwrap();
        matches!(namespace_info.configuration, AkdConfiguration::BitwardenV1Configuration);
        matches!(namespace_info.status, NamespaceStatus::Disabled);
    }
    
    #[test]
    fn test_status_transitions() {
        let disabled_config = NamespaceConfig {
            name: "test".to_string(),
            configuration_type: AkdConfigurationType::WhatsAppV1,
            log_directory: "logs/test".to_string(),
            starting_epoch: 0,
            status: ConfigNamespaceStatus::Disabled,
        };
        
        // Test: Online -> Disabled (should change)
        let online_existing = NamespaceInfo {
            configuration: AkdConfiguration::WhatsAppV1Configuration,
            name: "test".to_string(),
            log_directory: "logs/test".to_string(),
            last_verified_epoch: 5u64.into(),
            status: NamespaceStatus::Online,
        };
        
        let (info, changed) = disabled_config.to_namespace_info(Some(&online_existing)).unwrap();
        assert!(matches!(info.status, NamespaceStatus::Disabled));
        assert!(changed, "Should detect status change from Online to Disabled");
        
        // Test: Initialization -> Disabled (should change)  
        let init_existing = NamespaceInfo {
            configuration: AkdConfiguration::WhatsAppV1Configuration,
            name: "test".to_string(),
            log_directory: "logs/test".to_string(),
            last_verified_epoch: 5u64.into(),
            status: NamespaceStatus::Initialization,
        };
        
        let (info, changed) = disabled_config.to_namespace_info(Some(&init_existing)).unwrap();
        assert!(matches!(info.status, NamespaceStatus::Disabled));
        assert!(changed, "Should detect status change from Initialization to Disabled");
        
        // Test: SignatureLost -> should NOT change (preserve error state)
        let error_existing = NamespaceInfo {
            configuration: AkdConfiguration::WhatsAppV1Configuration,
            name: "test".to_string(),
            log_directory: "logs/test".to_string(),
            last_verified_epoch: 5u64.into(),
            status: NamespaceStatus::SignatureLost,
        };
        
        let (info, changed) = disabled_config.to_namespace_info(Some(&error_existing)).unwrap();
        assert!(matches!(info.status, NamespaceStatus::SignatureLost));
        assert!(!changed, "Should NOT change error states");
        
        // Test: SignatureVerificationFailed -> should NOT change (preserve error state)
        let verification_error_existing = NamespaceInfo {
            configuration: AkdConfiguration::WhatsAppV1Configuration,
            name: "test".to_string(),
            log_directory: "logs/test".to_string(),
            last_verified_epoch: 5u64.into(),
            status: NamespaceStatus::SignatureVerificationFailed,
        };
        
        let (info, changed) = disabled_config.to_namespace_info(Some(&verification_error_existing)).unwrap();
        assert!(matches!(info.status, NamespaceStatus::SignatureVerificationFailed));
        assert!(!changed, "Should NOT change error states");
    }
    
    #[test]
    fn test_resolve_status_transition() {
        
        // Test new namespace (None existing status)
        let (status, changed) = NamespaceConfig::resolve_status_transition(
            &ConfigNamespaceStatus::Online, 
            None
        );
        assert!(matches!(status, NamespaceStatus::Online));
        assert!(!changed, "New namespace should not count as changed");
        
        let (status, changed) = NamespaceConfig::resolve_status_transition(
            &ConfigNamespaceStatus::Disabled, 
            None
        );
        assert!(matches!(status, NamespaceStatus::Disabled));
        assert!(!changed, "New namespace should not count as changed");
        
        // Test error states are preserved
        let (status, changed) = NamespaceConfig::resolve_status_transition(
            &ConfigNamespaceStatus::Online,
            Some(&NamespaceStatus::SignatureLost)
        );
        assert!(matches!(status, NamespaceStatus::SignatureLost));
        assert!(!changed, "Error states should never be changed");
        
        let (status, changed) = NamespaceConfig::resolve_status_transition(
            &ConfigNamespaceStatus::Disabled,
            Some(&NamespaceStatus::SignatureVerificationFailed)
        );
        assert!(matches!(status, NamespaceStatus::SignatureVerificationFailed));
        assert!(!changed, "Error states should never be changed");
        
        // Test normal status transitions
        let (status, changed) = NamespaceConfig::resolve_status_transition(
            &ConfigNamespaceStatus::Disabled,
            Some(&NamespaceStatus::Online)
        );
        assert!(matches!(status, NamespaceStatus::Disabled));
        assert!(changed, "Online -> Disabled should be detected as change");
        
        let (status, changed) = NamespaceConfig::resolve_status_transition(
            &ConfigNamespaceStatus::Online,
            Some(&NamespaceStatus::Disabled)
        );
        assert!(matches!(status, NamespaceStatus::Online));
        assert!(changed, "Disabled -> Online should be detected as change");
        
        let (status, changed) = NamespaceConfig::resolve_status_transition(
            &ConfigNamespaceStatus::Disabled,
            Some(&NamespaceStatus::Initialization)
        );
        assert!(matches!(status, NamespaceStatus::Disabled));
        assert!(changed, "Initialization -> Disabled should be detected as change");
        
        // Test no change scenarios
        let (status, changed) = NamespaceConfig::resolve_status_transition(
            &ConfigNamespaceStatus::Online,
            Some(&NamespaceStatus::Online)
        );
        assert!(matches!(status, NamespaceStatus::Online));
        assert!(!changed, "Online -> Online should not be detected as change");
        
        let (status, changed) = NamespaceConfig::resolve_status_transition(
            &ConfigNamespaceStatus::Disabled,
            Some(&NamespaceStatus::Disabled)
        );
        assert!(matches!(status, NamespaceStatus::Disabled));
        assert!(!changed, "Disabled -> Disabled should not be detected as change");
    }
    
    #[test]
    fn test_storage_config_validation() {
        // Test InMemory - should always be valid
        let inmemory = StorageConfig::InMemory;
        assert!(inmemory.validate().is_ok());
        
        // Test File with valid directory that exists
        let file_valid = StorageConfig::File {
            directory: "/tmp".to_string(), // /tmp should exist on most systems
        };
        assert!(file_valid.validate().is_ok());
        
        // Test File with empty directory
        let file_empty = StorageConfig::File {
            directory: "".to_string(),
        };
        let result = file_empty.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
        
        // Test File with non-existent directory
        let file_nonexistent = StorageConfig::File {
            directory: "/this/directory/should/not/exist/hopefully/12345".to_string(),
        };
        let result = file_nonexistent.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
        
        // Test Azure with connection string in config
        let azure_with_conn = StorageConfig::Azure {
            account_name: "test".to_string(),
            container_name: "test".to_string(),
            connection_string: Some("DefaultEndpointsProtocol=https;AccountName=test;".to_string()),
        };
        assert!(azure_with_conn.validate().is_ok());
        
        // Test Azure without connection string (should fail)
        let azure_no_conn = StorageConfig::Azure {
            account_name: "test".to_string(),
            container_name: "test".to_string(),
            connection_string: None,
        };
        let result = azure_no_conn.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires connection_string"));
    }
}
 