mod in_memory_namespace_repository;
mod file_namespace_repository;

pub use in_memory_namespace_repository::InMemoryNamespaceRepository;
pub use file_namespace_repository::FileNamespaceRepository;

use thiserror::Error;

use crate::NamespaceInfo;
use std::{
    fmt::Debug,
};

#[derive(Debug, Error)]
pub enum NamespaceRepositoryError {
    #[error("Namespace not found: {0}")]
    NamespaceNotFound(String),
    #[error("{0}")]
    PersistenceError(#[from] NamespaceRepositoryPersistenceError),
}

#[derive(Debug, Error)]
#[error("Initialization error: {0}")]
pub struct NamespaceRepositoryInitializationError(String);

#[derive(Debug, Error)]
#[error("Persistence error: {0}")]
pub struct NamespaceRepositoryPersistenceError(String);

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
