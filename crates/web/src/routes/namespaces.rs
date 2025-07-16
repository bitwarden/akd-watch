use axum::Json;
use serde::{Deserialize, Serialize};

use crate::routes::audits::Epoch;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum NamespaceStatus {
    Online,
    Initialization,
    Disabled,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NamespaceInfo {
    log_directory: Option<String>,
    last_verified_epoch: Option<Epoch>,
    status: NamespaceStatus,
}

pub async fn handle_query_namespace(
    axum::extract::Path(namespace): axum::extract::Path<String>,
) -> Json<Option<NamespaceInfo>> {
    // Placeholder: Return list of blobs in the namespace
    Json(NamespaceInfo {
        log_directory: Some(format!("Namespace: {}", namespace)),
        last_verified_epoch: Some(Epoch::new(42)),
        status: NamespaceStatus::Online,
    }.into())
}

pub async fn handle_list_namespaces() -> Json<Vec<String>> {
    // Placeholder: Return list of namespaces
    Json(vec!["namespace1".to_string(), "namespace2".to_string()])
}
