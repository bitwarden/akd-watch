mod in_memory_namespace_repository;
mod file_namespace_repository;

pub use in_memory_namespace_repository::InMemoryNamespaceRepository;
pub use file_namespace_repository::FileNamespaceRepository;

use thiserror::Error;
use std::future::Future;

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

/// Enum wrapper to support different namespace repository implementations
/// 
/// This enum allows applications to work with different storage backends
/// for namespace information (File-based or InMemory) based on configuration.
#[derive(Clone, Debug)]
pub enum NamespaceStorage {
    File(FileNamespaceRepository),
    InMemory(InMemoryNamespaceRepository),
}

impl NamespaceRepository for NamespaceStorage {
    async fn get_namespace_info(
        &self,
        name: &str,
    ) -> Result<Option<crate::NamespaceInfo>> {
        match self {
            NamespaceStorage::File(repo) => repo.get_namespace_info(name).await,
            NamespaceStorage::InMemory(repo) => repo.get_namespace_info(name).await,
        }
    }

    async fn list_namespaces(&self) -> Result<Vec<crate::NamespaceInfo>> {
        match self {
            NamespaceStorage::File(repo) => repo.list_namespaces().await,
            NamespaceStorage::InMemory(repo) => repo.list_namespaces().await,
        }
    }

    async fn add_namespace(&mut self, info: crate::NamespaceInfo) -> Result<()> {
        match self {
            NamespaceStorage::File(repo) => repo.add_namespace(info).await,
            NamespaceStorage::InMemory(repo) => repo.add_namespace(info).await,
        }
    }

    async fn update_namespace(&mut self, info: crate::NamespaceInfo) -> Result<()> {
        match self {
            NamespaceStorage::File(repo) => repo.update_namespace(info).await,
            NamespaceStorage::InMemory(repo) => repo.update_namespace(info).await,
        }
    }

    async fn remove_namespace(&mut self, name: &str) -> Result<()> {
        match self {
            NamespaceStorage::File(repo) => repo.remove_namespace(name).await,
            NamespaceStorage::InMemory(repo) => repo.remove_namespace(name).await,
        }
    }
}
