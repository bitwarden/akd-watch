use tracing::{debug, instrument, trace};

use crate::{
    NamespaceInfo,
    storage::namespaces::{
        NamespaceRepository, NamespaceRepositoryError, NamespaceRepositoryInitializationError,
        NamespaceRepositoryPersistenceError,
    },
};
use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, RwLock},
};

#[derive(Clone, Debug)]
pub struct FileNamespaceRepository {
    file_path: String,
    namespaces: Arc<RwLock<HashMap<String, NamespaceInfo>>>,
}

impl FileNamespaceRepository {
    pub fn new(file_path: String) -> Self {
        // Load existing namespaces from file, if it exists
        let namespaces = if std::path::Path::new(&file_path).exists() {
            Self::load_file(&file_path).expect("Failed to load namespaces from file")
        } else {
            HashMap::new()
        };

        Self {
            file_path,
            namespaces: Arc::new(RwLock::new(namespaces)),
        }
    }

    fn load_file(
        file_path: &str,
    ) -> Result<HashMap<String, NamespaceInfo>, NamespaceRepositoryInitializationError> {
        // Read file contents
        let file_content = std::fs::read_to_string(file_path).map_err(|e| {
            NamespaceRepositoryInitializationError(format!("Failed to read file: {}", e))
        })?;
        debug!("Loaded namespace file content");
        // Parse file contents into NamespaceInfo entries
        let namespaces: HashMap<String, NamespaceInfo> = serde_json::from_str(&file_content)
            .map_err(|e| {
                NamespaceRepositoryInitializationError(format!("Failed to parse file: {}", e))
            })?;
        debug!("Parsed {} namespaces from file", namespaces.len());
        Ok(namespaces)
    }

    fn persist(&self, locked_namespaces: &HashMap<String, NamespaceInfo>) -> Result<(), NamespaceRepositoryPersistenceError> {
        trace!("Persisting namespaces to file: {}", self.file_path);
        let serialized = serde_json::to_string(&*locked_namespaces).map_err(|e| {
            NamespaceRepositoryPersistenceError(format!("Failed to serialize namespaces: {}", e))
        })?;
        trace!("Serialized namespaces: {}", serialized);
        std::fs::write(&self.file_path, serialized).map_err(|e| {
            NamespaceRepositoryPersistenceError(format!("Failed to write to file: {}", e))
        })?;
        debug!("Successfully persisted {} namespaces", locked_namespaces.len());
        Ok(())
    }
}

impl NamespaceRepository for FileNamespaceRepository {
    #[instrument(level = "debug", skip(self))]
    async fn get_namespace_info(
        &self,
        name: &str,
    ) -> Result<Option<NamespaceInfo>, NamespaceRepositoryError> {
        let namespaces = self.namespaces.read().expect("Namespaces lock poisoned");
        Ok(namespaces.get(name).cloned())
    }

    #[instrument(level = "debug", skip(self))]
    async fn list_namespaces(&self) -> Result<Vec<NamespaceInfo>, NamespaceRepositoryError> {
        let namespaces = self.namespaces.read().expect("Namespaces lock poisoned");
        Ok(namespaces.values().cloned().collect())
    }

    #[instrument(level = "info", skip(self))]
    async fn add_namespace(&mut self, info: NamespaceInfo) -> Result<(), NamespaceRepositoryError> {
        let mut namespaces = self.namespaces.write().expect("Namespaces lock poisoned");
        namespaces.insert(info.name.clone(), info);
        self.persist(&namespaces)?;
        Ok(())
    }

    #[instrument(level = "info", skip(self))]
    async fn update_namespace(
        &mut self,
        info: NamespaceInfo,
    ) -> Result<(), NamespaceRepositoryError> {
        let mut namespaces = self.namespaces.write().expect("Namespaces lock poisoned");
        if namespaces.contains_key(&info.name) {
            namespaces.insert(info.name.clone(), info);
            self.persist(&namespaces)?;
            Ok(())
        } else {
            Err(NamespaceRepositoryError::NamespaceNotFound(info.name))
        }
    }

    #[instrument(level = "info", skip(self))]
    async fn remove_namespace(&mut self, name: &str) -> Result<(), NamespaceRepositoryError> {
        let mut namespaces = self.namespaces.write().expect("Namespaces lock poisoned");
        if namespaces.remove(name).is_some() {
            self.persist(&namespaces)?;
            Ok(())
        } else {
            Err(NamespaceRepositoryError::NamespaceNotFound(
                name.to_string(),
            ))
        }
    }
}
