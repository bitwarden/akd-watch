use akd_watch_common::{storage::signatures::SignatureRepository, web_api::SignatureResponse};
use axum::Json;
use tracing::{info, instrument, trace};

use crate::{AppState, error::ApiError};

#[instrument(skip_all, fields(namespace = %namespace, epoch))]
pub async fn audit_query_handler(
    axum::extract::State(AppState {
        signature_storage, ..
    }): axum::extract::State<AppState>,
    axum::extract::Path((namespace, epoch)): axum::extract::Path<(String, String)>,
) -> Result<Json<Option<SignatureResponse>>, ApiError> {
    info!(
        "Handling audit query for namespace: {}, epoch: {}",
        namespace, epoch
    );
    let epoch: u64 = epoch
        .parse()
        .map_err(|_| ApiError::BadRequest("epoch is not an integer".to_string()))?;
    let namespace_signature_storage =
        signature_storage
            .get(&namespace)
            .ok_or(ApiError::BadRequest(format!(
                "namespace {namespace} not found"
            )))?;
    trace!(namespace, epoch, "Found namespace storage for audit query");

    match namespace_signature_storage.get_signature(&epoch).await {
        Ok(Some(maybe_sig)) => Ok(Json(Some(maybe_sig.into()))),
        Ok(None) => {
            info!(
                "No signature found for namespace {} at epoch {}",
                namespace, epoch
            );
            Ok(Json(None))
        }
        Err(e) => {
            tracing::error!(
                "Failed to get signature for namespace {} at epoch {}: {}",
                namespace,
                epoch,
                e
            );
            Err(ApiError::Internal)
        }
    }
}
