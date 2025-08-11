use std::{collections::HashMap, sync::{Arc, RwLock}};
use crate::{NamespaceInfo, storage::namespace_repository::{NamespaceRepository, NamespaceRepositoryError}};

/// Mock namespace repository for testing
#[derive(Clone, Debug)]
pub struct MockNamespaceRepository {
    namespaces: Arc<RwLock<HashMap<String, NamespaceInfo>>>,
    should_fail: Arc<RwLock<bool>>,
}

impl MockNamespaceRepository {
    pub fn new() -> Self {
        Self {
            namespaces: Arc::new(RwLock::new(HashMap::new())),
            should_fail: Arc::new(RwLock::new(false)),
        }
    }

    /// Add a namespace for testing
    pub fn add_test_namespace(&mut self, namespace: NamespaceInfo) {
        self.namespaces.write().unwrap().insert(namespace.name.clone(), namespace);
    }

    /// Remove a namespace for testing
    pub fn remove_test_namespace(&mut self, name: &str) {
        self.namespaces.write().unwrap().remove(name);
    }

    /// Set whether operations should fail
    pub fn set_should_fail(&mut self, should_fail: bool) {
        *self.should_fail.write().unwrap() = should_fail;
    }

    /// Get the number of stored namespaces
    pub fn namespace_count(&self) -> usize {
        self.namespaces.read().unwrap().len()
    }
}

impl NamespaceRepository for MockNamespaceRepository {
    fn get_namespace_info(&self, name: &str) -> impl std::future::Future<Output = Result<Option<NamespaceInfo>, NamespaceRepositoryError>> + Send {
        let should_fail = *self.should_fail.read().unwrap();
        let result = if should_fail {
            Err(NamespaceRepositoryError::Custom("Mock failure".to_string()))
        } else {
            self.namespaces
                .read()
                .map_err(|_| NamespaceRepositoryError::Custom("Failed to acquire read lock".to_string()))
                .map(|namespaces| namespaces.get(name).cloned())
        };
        async move { result }
    }

    fn list_namespaces(&self) -> impl std::future::Future<Output = Result<Vec<NamespaceInfo>, NamespaceRepositoryError>> + Send {
        let should_fail = *self.should_fail.read().unwrap();
        let result = if should_fail {
            Err(NamespaceRepositoryError::Custom("Mock failure".to_string()))
        } else {
            self.namespaces
                .read()
                .map_err(|_| NamespaceRepositoryError::Custom("Failed to acquire read lock".to_string()))
                .map(|namespaces| namespaces.values().cloned().collect())
        };
        async move { result }
    }

    fn add_namespace(&mut self, info: NamespaceInfo) -> impl std::future::Future<Output = Result<(), NamespaceRepositoryError>> + Send {
        let should_fail = *self.should_fail.read().unwrap();
        let result = if should_fail {
            Err(NamespaceRepositoryError::Custom("Mock failure".to_string()))
        } else {
            self.namespaces
                .write()
                .map_err(|_| NamespaceRepositoryError::Custom("Failed to acquire write lock".to_string()))
                .map(|mut namespaces| {
                    namespaces.insert(info.name.clone(), info);
                })
        };
        async move { result }
    }

    fn update_namespace(&mut self, info: NamespaceInfo) -> impl std::future::Future<Output = Result<(), NamespaceRepositoryError>> + Send {
        let should_fail = *self.should_fail.read().unwrap();
        let result = if should_fail {
            Err(NamespaceRepositoryError::Custom("Mock failure".to_string()))
        } else {
            self.namespaces
                .write()
                .map_err(|_| NamespaceRepositoryError::Custom("Failed to acquire write lock".to_string()))
                .map(|mut namespaces| {
                    namespaces.insert(info.name.clone(), info);
                })
        };
        async move { result }
    }

    fn remove_namespace(&mut self, name: &str) -> impl std::future::Future<Output = Result<(), NamespaceRepositoryError>> + Send {
        let name = name.to_string();
        let should_fail = *self.should_fail.read().unwrap();
        let result = if should_fail {
            Err(NamespaceRepositoryError::Custom("Mock failure".to_string()))
        } else {
            self.namespaces
                .write()
                .map_err(|_| NamespaceRepositoryError::Custom("Failed to acquire write lock".to_string()))
                .map(|mut namespaces| {
                    namespaces.remove(&name);
                })
        };
        async move { result }
    }
}
