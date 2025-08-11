use crate::{
    akd_configurations::AkdConfiguration,
    storage::{whatsapp_akd_storage::WhatsAppAkdStorage, AkdStorage}, NamespaceInfo,
};

#[cfg(any(test, feature = "testing"))]
use crate::storage::test_akd_storage::TestAkdStorage;

/// Enum representing all possible AKD storage implementations
/// This enum exists because AkdStorage requires Clone, making it not object-safe
#[derive(Clone, Debug)]
pub enum AkdStorageImpl {
    WhatsApp(WhatsAppAkdStorage),
    #[cfg(any(test, feature = "testing"))]
    Test(TestAkdStorage),
}

impl std::fmt::Display for AkdStorageImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AkdStorageImpl::WhatsApp(storage) => std::fmt::Display::fmt(storage, f),
            #[cfg(any(test, feature = "testing"))]
            AkdStorageImpl::Test(storage) => std::fmt::Display::fmt(storage, f),
        }
    }
}

impl AkdStorage for AkdStorageImpl {
    async fn has_proof(&self, epoch: u64) -> bool {
        match self {
            AkdStorageImpl::WhatsApp(storage) => storage.has_proof(epoch).await,
            #[cfg(any(test, feature = "testing"))]
            AkdStorageImpl::Test(storage) => storage.has_proof(epoch).await,
        }
    }

    async fn get_proof_name(&self, epoch: u64) -> Result<akd::local_auditing::AuditBlobName, crate::storage::AkdStorageError> {
        match self {
            AkdStorageImpl::WhatsApp(storage) => storage.get_proof_name(epoch).await,
            #[cfg(any(test, feature = "testing"))]
            AkdStorageImpl::Test(storage) => storage.get_proof_name(epoch).await,
        }
    }

    async fn get_proof(&self, name: &akd::local_auditing::AuditBlobName) -> Result<akd::local_auditing::AuditBlob, crate::storage::AkdStorageError> {
        match self {
            AkdStorageImpl::WhatsApp(storage) => storage.get_proof(name).await,
            #[cfg(any(test, feature = "testing"))]
            AkdStorageImpl::Test(storage) => storage.get_proof(name).await,
        }
    }
}

/// Factory for creating AKD storage implementations based on configuration
pub struct AkdStorageFactory;

impl AkdStorageFactory {
    /// Create an AKD storage implementation based on the given configuration
pub fn create_storage(namespace_info: &NamespaceInfo) -> AkdStorageImpl {
        match namespace_info.configuration {
            AkdConfiguration::WhatsAppV1Configuration => AkdStorageImpl::WhatsApp(WhatsAppAkdStorage::new()),
            #[cfg(any(test, feature = "testing"))]
            AkdConfiguration::TestConfiguration => AkdStorageImpl::Test(TestAkdStorage::new()),
            _ => todo!("Unsupported configuration: {:?}", namespace_info.configuration),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factory_creates_whatsapp_storage() {
        let namespace = NamespaceInfo {
            name: "whatsapp".to_string(),
            configuration: AkdConfiguration::WhatsAppV1Configuration,
            log_directory: "https://d1tfr3x7n136ak.cloudfront.net/".to_string(),
            starting_epoch: 1000000.into(),
            status: crate::NamespaceStatus::Online,
            last_verified_epoch: Some(1000000.into()),
        };

        let storage = AkdStorageFactory::create_storage(&namespace);
        assert!(matches!(storage, AkdStorageImpl::WhatsApp(_)));
        assert!(format!("{}", storage).contains("WhatsApp"));
    }

    #[test]
    fn test_factory_creates_test_storage() {
        let namespace =         NamespaceInfo {
            name: "test".to_string(),
            configuration: AkdConfiguration::TestConfiguration,
            log_directory: "https://example.com/".to_string(),
            starting_epoch: 1000000.into(),
            status: crate::NamespaceStatus::Online,
            last_verified_epoch: Some(1000000.into()),
        };

        let storage = AkdStorageFactory::create_storage(&namespace);
        assert!(matches!(storage, AkdStorageImpl::Test(_)));
        assert!(format!("{}", storage).contains("Test"));
    }
}
