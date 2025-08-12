use thiserror::Error;

use crate::NamespaceInfo;
use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, RwLock},
};

#[derive(Debug, Error)]
pub enum NamespaceRepositoryError {
    #[error("{0}")]
    Custom(String),
}

type Result<T> = std::result::Result<T, NamespaceRepositoryError>;

pub trait NamespaceRepository: Clone + Send + Sync {
    fn get_namespace_info(
        &self,
        name: &str,
    ) -> impl Future<Output = Result<Option<NamespaceInfo>>> + Send;
    fn list_namespaces(&self) -> impl Future<Output = Result<Vec<NamespaceInfo>>> + Send;
    fn add_namespace(&mut self, info: NamespaceInfo) -> impl Future<Output = Result<()>> + Send;
    fn update_namespace(&mut self, info: NamespaceInfo) -> impl Future<Output = Result<()>> + Send;
    fn remove_namespace(&mut self, name: &str) -> impl Future<Output = Result<()>> + Send;
}

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
            let namespaces = namespaces.read().map_err(|_| {
                NamespaceRepositoryError::Custom("Failed to acquire read lock".to_string())
            })?;
            Ok(namespaces.get(name).cloned())
        }
    }

    fn list_namespaces(&self) -> impl Future<Output = Result<Vec<NamespaceInfo>>> + Send {
        let namespaces = self.namespaces.clone();
        async move {
            let namespaces = namespaces.read().map_err(|_| {
                NamespaceRepositoryError::Custom("Failed to acquire read lock".to_string())
            })?;
            Ok(namespaces.values().cloned().collect())
        }
    }

    fn add_namespace(&mut self, info: NamespaceInfo) -> impl Future<Output = Result<()>> + Send {
        let namespaces = self.namespaces.clone();
        async move {
            let mut namespaces = namespaces.write().map_err(|_| {
                NamespaceRepositoryError::Custom("Failed to acquire write lock".to_string())
            })?;
            namespaces.insert(info.name.clone(), info);
            Ok(())
        }
    }

    fn update_namespace(&mut self, info: NamespaceInfo) -> impl Future<Output = Result<()>> + Send {
        let namespaces = self.namespaces.clone();
        async move {
            let mut namespaces = namespaces.write().map_err(|_| {
                NamespaceRepositoryError::Custom("Failed to acquire write lock".to_string())
            })?;
            if namespaces.contains_key(&info.name) {
                namespaces.insert(info.name.clone(), info);
                Ok(())
            } else {
                Err(NamespaceRepositoryError::Custom(format!(
                    "Namespace '{}' does not exist",
                    info.name
                )))
            }
        }
    }

    fn remove_namespace(&mut self, name: &str) -> impl Future<Output = Result<()>> + Send {
        let namespaces = self.namespaces.clone();
        async move {
            let mut namespaces = namespaces.write().map_err(|_| {
                NamespaceRepositoryError::Custom("Failed to acquire write lock".to_string())
            })?;
            if namespaces.remove(name).is_some() {
                Ok(())
            } else {
                Err(NamespaceRepositoryError::Custom(format!(
                    "Namespace '{}' does not exist",
                    name
                )))
            }
        }
    }
}
