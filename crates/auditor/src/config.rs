use akd_watch_common::{
    Epoch, NamespaceInfo, NamespaceStatus,
    akd_configurations::AkdConfiguration,
    config::{NamespaceStorageConfig, SignatureStorageConfig, SigningConfig},
};
use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Default constant for sleep duration between audit cycles.= 30 seconds
const DEFAULT_SLEEP_SECONDS: u64 = 30;

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
    /// Defaults given by [`DEFAULT_SLEEP_SECONDS`]
    #[serde(default = "default_sleep_seconds")]
    pub sleep_seconds: u64,

    /// Namespace configurations to audit
    pub namespaces: Vec<NamespaceConfig>,

    /// Namespace storage configuration
    pub namespace_storage: NamespaceStorageConfig,

    /// Signing key configuration
    pub signing: SigningConfig,

    /// Storage configuration
    pub signature_storage: SignatureStorageConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NamespaceConfig {
    /// Name of the namespace
    pub name: String,

    /// Configuration type
    pub configuration_type: AkdConfigurationType,

    /// Url to query for proofs
    pub log_directory: String,

    /// Starting epoch for auditing (only used if no existing namespace info found)
    #[serde(default)]
    pub starting_epoch: u64,

    /// Status
    pub status: ConfigNamespaceStatus,
}

impl AuditorConfig {
    /// Load configuration from multiple sources in order of priority:
    /// 1. Configuration file (config.toml, config.yaml, config.json)
    /// 2. Environment variables (prefixed with AKD_WATCH_)
    pub fn load() -> Result<Self, ConfigError> {
        let config = Config::builder()
            // Start with default config file
            .add_source(File::with_name("config").required(false))
            // Add environment variables with prefix "AKD_WATCH_"
            .add_source(Environment::with_prefix("AKD_WATCH").separator("_"))
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
            .add_source(Environment::with_prefix("AKD_WATCH").separator("_"))
            .build()?;

        let auditor_config: Self = config.try_deserialize()?;

        auditor_config.validate()?;

        Ok(auditor_config)
    }

    /// Validate the entire auditor configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate storage configuration
        self.namespace_storage.validate()?;
        self.signature_storage.validate()?;
        self.signing.validate()?;

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
    /// Returns a tuple of (NamespaceInfo, bool) where the bool indicates if there are updates to persist for the existing namespace.
    /// New namespaces will not count as a change.
    pub fn to_namespace_info(
        &self,
        existing_namespace_info: Option<&NamespaceInfo>,
    ) -> Result<(NamespaceInfo, bool), ConfigError> {
        let configuration = match self.configuration_type {
            AkdConfigurationType::WhatsAppV1 => AkdConfiguration::WhatsAppV1Configuration,
            AkdConfigurationType::BitwardenV1 => AkdConfiguration::BitwardenV1Configuration,
        };

        // Handle status transitions
        let (status, status_changed) = Self::resolve_status_transition(
            &self.status,
            existing_namespace_info.map(|info| &info.status),
        );

        // Use existing last_verified_epoch if available
        let existing_last_verified_epoch = existing_namespace_info
            .map(|info| info.last_verified_epoch)
            .flatten();
        let (last_verified_epoch, last_verified_epoch_changed) =
            Self::resolve_last_verified_epoch(self.starting_epoch, existing_last_verified_epoch);

        // Always use the starting_epoch from config (may have been updated)
        let starting_epoch = self.starting_epoch.into();

        let namespace_info = NamespaceInfo {
            configuration,
            name: self.name.clone(),
            log_directory: self.log_directory.clone(),
            last_verified_epoch,
            starting_epoch,
            status,
        };

        let changed = status_changed || last_verified_epoch_changed;

        Ok((namespace_info, changed))
    }

    /// Resolve status transitions based on configuration and existing status
    ///
    /// Returns (new_status, status_changed)
    fn resolve_status_transition(
        config_status: &ConfigNamespaceStatus,
        existing_status: Option<&NamespaceStatus>,
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

    /// Resolves the last_verified_epoch based on config starting_epoch and existing value
    /// Alters the last_verified_epoch only if the existing value is less than the config starting_epoch,
    /// and indicates this override with a true boolean in the return tuple.
    fn resolve_last_verified_epoch(
        config_starting_epoch: u64,
        existing_last_verified_epoch: Option<Epoch>,
    ) -> (Option<Epoch>, bool) {
        match existing_last_verified_epoch {
            Some(epoch) if epoch.value() >= &config_starting_epoch => (Some(epoch), false),
            Some(epoch) if epoch.value() < &config_starting_epoch => {
                // If existing epoch is less than config starting epoch, treat as a new namespace
                (None, true)
            }
            None => (None, false),
            Some(_) => panic!("Unreachable case"),
        }
    }
}

// Default values for serde defaults
fn default_sleep_seconds() -> u64 {
    DEFAULT_SLEEP_SECONDS
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
        assert_eq!(namespace_info.last_verified_epoch, None); // No existing info, so None
        assert_eq!(namespace_info.starting_epoch, 5u64.into());
        assert!(!status_changed); // New namespace, so no change

        // Test with existing namespace info (should preserve existing last_verified_epoch)
        let existing_info = NamespaceInfo {
            configuration: AkdConfiguration::WhatsAppV1Configuration,
            name: "test".to_string(),
            log_directory: "logs/test".to_string(),
            last_verified_epoch: Some(10u64.into()),
            starting_epoch: 5u64.into(),
            status: NamespaceStatus::Online,
        };

        let (namespace_info, status_changed) = namespace_config
            .to_namespace_info(Some(&existing_info))
            .unwrap();
        assert_eq!(namespace_info.name, "test");
        assert_eq!(namespace_info.last_verified_epoch, Some(10u64.into())); // Should preserve existing value
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
        matches!(
            namespace_info.configuration,
            AkdConfiguration::WhatsAppV1Configuration
        );

        let bitwarden_config = NamespaceConfig {
            name: "test".to_string(),
            configuration_type: AkdConfigurationType::BitwardenV1,
            log_directory: "logs/test".to_string(),
            starting_epoch: 0,
            status: ConfigNamespaceStatus::Disabled,
        };

        let (namespace_info, _) = bitwarden_config.to_namespace_info(None).unwrap();
        matches!(
            namespace_info.configuration,
            AkdConfiguration::BitwardenV1Configuration
        );
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
            last_verified_epoch: Some(5u64.into()),
            starting_epoch: 1u64.into(),
            status: NamespaceStatus::Online,
        };

        let (info, changed) = disabled_config
            .to_namespace_info(Some(&online_existing))
            .unwrap();
        assert!(matches!(info.status, NamespaceStatus::Disabled));
        assert!(
            changed,
            "Should detect status change from Online to Disabled"
        );

        // Test: Initialization -> Disabled (should change)
        let init_existing = NamespaceInfo {
            configuration: AkdConfiguration::WhatsAppV1Configuration,
            name: "test".to_string(),
            log_directory: "logs/test".to_string(),
            last_verified_epoch: Some(5u64.into()),
            starting_epoch: 1u64.into(),
            status: NamespaceStatus::Initialization,
        };

        let (info, changed) = disabled_config
            .to_namespace_info(Some(&init_existing))
            .unwrap();
        assert!(matches!(info.status, NamespaceStatus::Disabled));
        assert!(
            changed,
            "Should detect status change from Initialization to Disabled"
        );

        // Test: SignatureLost -> should NOT change (preserve error state)
        let error_existing = NamespaceInfo {
            configuration: AkdConfiguration::WhatsAppV1Configuration,
            name: "test".to_string(),
            log_directory: "logs/test".to_string(),
            last_verified_epoch: Some(5u64.into()),
            starting_epoch: 1u64.into(),
            status: NamespaceStatus::SignatureLost,
        };

        let (info, changed) = disabled_config
            .to_namespace_info(Some(&error_existing))
            .unwrap();
        assert!(matches!(info.status, NamespaceStatus::SignatureLost));
        assert!(!changed, "Should NOT change error states");

        // Test: SignatureVerificationFailed -> should NOT change (preserve error state)
        let verification_error_existing = NamespaceInfo {
            configuration: AkdConfiguration::WhatsAppV1Configuration,
            name: "test".to_string(),
            log_directory: "logs/test".to_string(),
            last_verified_epoch: Some(5u64.into()),
            starting_epoch: 1u64.into(),
            status: NamespaceStatus::SignatureVerificationFailed,
        };

        let (info, changed) = disabled_config
            .to_namespace_info(Some(&verification_error_existing))
            .unwrap();
        assert!(matches!(
            info.status,
            NamespaceStatus::SignatureVerificationFailed
        ));
        assert!(!changed, "Should NOT change error states");
    }

    #[test]
    fn test_resolve_status_transition() {
        // Test new namespace (None existing status)
        let (status, changed) =
            NamespaceConfig::resolve_status_transition(&ConfigNamespaceStatus::Online, None);
        assert!(matches!(status, NamespaceStatus::Online));
        assert!(!changed, "New namespace should not count as changed");

        let (status, changed) =
            NamespaceConfig::resolve_status_transition(&ConfigNamespaceStatus::Disabled, None);
        assert!(matches!(status, NamespaceStatus::Disabled));
        assert!(!changed, "New namespace should not count as changed");

        // Test error states are preserved
        let (status, changed) = NamespaceConfig::resolve_status_transition(
            &ConfigNamespaceStatus::Online,
            Some(&NamespaceStatus::SignatureLost),
        );
        assert!(matches!(status, NamespaceStatus::SignatureLost));
        assert!(!changed, "Error states should never be changed");

        let (status, changed) = NamespaceConfig::resolve_status_transition(
            &ConfigNamespaceStatus::Disabled,
            Some(&NamespaceStatus::SignatureVerificationFailed),
        );
        assert!(matches!(
            status,
            NamespaceStatus::SignatureVerificationFailed
        ));
        assert!(!changed, "Error states should never be changed");

        // Test normal status transitions
        let (status, changed) = NamespaceConfig::resolve_status_transition(
            &ConfigNamespaceStatus::Disabled,
            Some(&NamespaceStatus::Online),
        );
        assert!(matches!(status, NamespaceStatus::Disabled));
        assert!(changed, "Online -> Disabled should be detected as change");

        let (status, changed) = NamespaceConfig::resolve_status_transition(
            &ConfigNamespaceStatus::Online,
            Some(&NamespaceStatus::Disabled),
        );
        assert!(matches!(status, NamespaceStatus::Online));
        assert!(changed, "Disabled -> Online should be detected as change");

        let (status, changed) = NamespaceConfig::resolve_status_transition(
            &ConfigNamespaceStatus::Disabled,
            Some(&NamespaceStatus::Initialization),
        );
        assert!(matches!(status, NamespaceStatus::Disabled));
        assert!(
            changed,
            "Initialization -> Disabled should be detected as change"
        );

        // Test no change scenarios
        let (status, changed) = NamespaceConfig::resolve_status_transition(
            &ConfigNamespaceStatus::Online,
            Some(&NamespaceStatus::Online),
        );
        assert!(matches!(status, NamespaceStatus::Online));
        assert!(
            !changed,
            "Online -> Online should not be detected as change"
        );

        let (status, changed) = NamespaceConfig::resolve_status_transition(
            &ConfigNamespaceStatus::Disabled,
            Some(&NamespaceStatus::Disabled),
        );
        assert!(matches!(status, NamespaceStatus::Disabled));
        assert!(
            !changed,
            "Disabled -> Disabled should not be detected as change"
        );
    }
}
