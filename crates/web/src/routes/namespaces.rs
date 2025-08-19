use akd_watch_common::{NamespaceInfo, storage::namespaces::NamespaceRepository};
use axum::Json;
use tracing::{info, instrument};

use crate::{error::ApiError, routes::AppState};

#[instrument(skip_all, fields(namespace))]
pub async fn namespace_query_handler(
    axum::extract::State(AppState {
        namespace_storage, ..
    }): axum::extract::State<AppState>,
    axum::extract::Path(namespace): axum::extract::Path<String>,
) -> Result<Json<Option<NamespaceInfo>>, ApiError> {
    info!("Handling namespace query for namespace: {}", namespace);
    match namespace_storage.get_namespace_info(&namespace).await {
        Ok(info) => Ok(Json(info)),
        Err(e) => {
            tracing::error!("Failed to get namespace info: {}", e);
            Err(ApiError::Internal)
        }
    }
}

#[instrument(skip_all)]
pub async fn list_namespaces_handler(
    axum::extract::State(AppState {
        namespace_storage, ..
    }): axum::extract::State<AppState>,
) -> Result<Json<Vec<NamespaceInfo>>, ApiError> {
    info!("Listing all namespaces");
    match namespace_storage.list_namespaces().await {
        Ok(namespaces) => Ok(Json(namespaces)),
        Err(e) => {
            tracing::error!("Failed to list namespaces: {}", e);
            Err(ApiError::Internal)
        }
    }
}
