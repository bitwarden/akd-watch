use crate::{
    NamespaceInfo,
    storage::namespaces::{NamespaceRepository, NamespaceRepositoryError},
};
use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, RwLock},
};

type Result<T> = std::result::Result<T, NamespaceRepositoryError>;

#[derive(Clone, Debug)]
pub struct InMemoryNamespaceRepository {
    namespaces: Arc<RwLock<HashMap<String, NamespaceInfo>>>,
}

impl InMemoryNamespaceRepository {
    pub fn new() -> Self {
        Self {
            namespaces: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl NamespaceRepository for InMemoryNamespaceRepository {
    fn get_namespace_info(
        &self,
        name: &str,
    ) -> impl Future<Output = Result<Option<NamespaceInfo>>> + Send {
        let namespaces = self.namespaces.clone();
        async move {
            let namespaces = namespaces.read().expect("Namespaces lock poisoned");
            Ok(namespaces.get(name).cloned())
        }
    }

    fn list_namespaces(&self) -> impl Future<Output = Result<Vec<NamespaceInfo>>> + Send {
        let namespaces = self.namespaces.clone();
        async move {
            let namespaces = namespaces.read().expect("Namespaces lock poisoned");
            Ok(namespaces.values().cloned().collect())
        }
    }

    fn add_namespace(&mut self, info: NamespaceInfo) -> impl Future<Output = Result<()>> + Send {
        let namespaces = self.namespaces.clone();
        async move {
            let mut namespaces = namespaces.write().expect("Namespaces lock poisoned");
            namespaces.insert(info.name.clone(), info);
            Ok(())
        }
    }

    fn update_namespace(&mut self, info: NamespaceInfo) -> impl Future<Output = Result<()>> + Send {
        let namespaces = self.namespaces.clone();
        async move {
            let mut namespaces = namespaces.write().expect("Namespaces lock poisoned");
            if namespaces.contains_key(&info.name) {
                namespaces.insert(info.name.clone(), info);
                Ok(())
            } else {
                Err(NamespaceRepositoryError::NamespaceNotFound(info.name))
            }
        }
    }

    fn remove_namespace(&mut self, name: &str) -> impl Future<Output = Result<()>> + Send {
        let namespaces = self.namespaces.clone();
        async move {
            let mut namespaces = namespaces.write().expect("Namespaces lock poisoned");
            if namespaces.remove(name).is_some() {
                Ok(())
            } else {
                Err(NamespaceRepositoryError::NamespaceNotFound(
                    name.to_string(),
                ))
            }
        }
    }
}
