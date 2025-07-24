use crate::NamespaceInfo;
use std::fmt::Debug;

pub trait NamespaceRepository: Clone + Debug + Send + Sync {
    fn get_namespace_info(&self, name: &str) -> impl Future<Output = Option<NamespaceInfo>> + Send;
    fn list_namespaces(&self) -> impl Future<Output = Vec<NamespaceInfo>> + Send;
    fn add_namespace(&mut self, info: NamespaceInfo) -> impl Future<Output = ()> + Send;
    fn update_namespace(&mut self, info: NamespaceInfo) -> impl Future<Output = ()> + Send;
    fn remove_namespace(&mut self, name: &str) -> impl Future<Output = ()> + Send;
}
