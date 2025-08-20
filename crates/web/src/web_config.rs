use akd_watch_common::config::{NamespaceStorageConfig, SignatureStorageConfig, VerifyingConfig};
use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};

fn default_bind_address() -> String {
    "127.0.0.1:3000".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WebConfig {
    /// Address to bind the web server to
    /// Defaults to 127.0.0.1:3000
    #[serde(default = "default_bind_address")]
    pub bind_address: String,

    /// Configuration for namespace storage
    pub namespace_storage: NamespaceStorageConfig,

    /// Configuration for signature storage
    pub signature_storage: SignatureStorageConfig,

    /// Configuration for verifying keys
    pub signing: VerifyingConfig,
}

impl WebConfig {
    pub fn load() -> Result<Self, ConfigError> {
        let config = Config::builder()
            .add_source(File::with_name("config").required(false))
            .add_source(Environment::with_prefix("AKD_WATCH").separator("_"))
            .build()?;
        let web_config = config.try_deserialize::<WebConfig>()?;

        web_config.validate()?;

        Ok(web_config)
    }

    /// Validate that the web configuration is complete and usable
    pub fn validate(&self) -> Result<(), ConfigError> {
        if let Err(e) = self.bind_address.parse::<std::net::SocketAddr>() {
            return Err(ConfigError::Message(format!(
                "Web bind_address is not a valid socket address: {e}"
            )));
        }

        self.namespace_storage.validate()?;
        self.signature_storage.validate()?;

        Ok(())
    }

    /// Get the socket address to bind the web server to
    /// Will panic if the configured bind_address string is not valid
    pub fn socket_addr(&self) -> std::net::SocketAddr {
        self.bind_address
            .parse()
            .expect("Failed to parse bind address")
    }
}
