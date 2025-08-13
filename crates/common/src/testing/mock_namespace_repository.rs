use crate::{
    NamespaceInfo,
    storage::namespaces::{NamespaceRepository, NamespaceRepositoryError},
};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

/// Mock namespace repository for testing
#[derive(Clone, Debug)]
pub struct MockNamespaceRepository {
    namespaces: Arc<RwLock<HashMap<String, NamespaceInfo>>>,
}

impl MockNamespaceRepository {
    pub fn new() -> Self {
        Self {
            namespaces: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a namespace for testing
    pub fn add_test_namespace(&mut self, namespace: NamespaceInfo) {
        self.namespaces
            .write()
            .unwrap()
            .insert(namespace.name.clone(), namespace);
    }

    /// Remove a namespace for testing
    pub fn remove_test_namespace(&mut self, name: &str) {
        self.namespaces.write().unwrap().remove(name);
    }

    /// Get the number of stored namespaces
    pub fn namespace_count(&self) -> usize {
        self.namespaces.read().unwrap().len()
    }
}

impl NamespaceRepository for MockNamespaceRepository {
    fn get_namespace_info(
        &self,
        name: &str,
    ) -> impl std::future::Future<Output = Result<Option<NamespaceInfo>, NamespaceRepositoryError>> + Send
    {
        let result = {
            Ok(self
                .namespaces
                .read()
                .expect("Namespaces lock poisoned")
                .get(name)
                .cloned())
        };
        async move { result }
    }

    fn list_namespaces(
        &self,
    ) -> impl std::future::Future<Output = Result<Vec<NamespaceInfo>, NamespaceRepositoryError>> + Send
    {
        let result = {
            Ok(self
                .namespaces
                .read()
                .expect("Namespaces lock poisoned")
                .values()
                .cloned()
                .collect())
        };
        async move { result }
    }

    fn add_namespace(
        &mut self,
        info: NamespaceInfo,
    ) -> impl std::future::Future<Output = Result<(), NamespaceRepositoryError>> + Send {
        let result = {
            self.namespaces
                .write()
                .expect("Namespaces lock poisoned")
                .insert(info.name.clone(), info);
            Ok(())
        };
        async move { result }
    }

    fn update_namespace(
        &mut self,
        info: NamespaceInfo,
    ) -> impl std::future::Future<Output = Result<(), NamespaceRepositoryError>> + Send {
        let result = {
            self.namespaces
                .write()
                .expect("Namespaces lock poisoned")
                .insert(info.name.clone(), info);
            Ok(())
        };
        async move { result }
    }

    fn remove_namespace(
        &mut self,
        name: &str,
    ) -> impl std::future::Future<Output = Result<(), NamespaceRepositoryError>> + Send {
        let name = name.to_string();
        let result = {
            self.namespaces
                .write()
                .expect("Namespaces lock poisoned")
                .remove(&name);
            Ok(())
        };
        async move { result }
    }
}
