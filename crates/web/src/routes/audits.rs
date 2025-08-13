use crate::error::ApiError;
use akd_watch_common::{AuditVersion, Ciphersuite, Epoch};
use axum::Json;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct SignatureResponse {
    version: AuditVersion,
    ciphersuite: Ciphersuite,
    namespace: String,
    timestamp: u64,
    epoch: Epoch,
    digest: Vec<u8>,
    signature: Vec<u8>,
    key_id: Option<u8>,
    serialized_message: Option<Vec<u8>>,
}

pub async fn handle_audit_query(
    axum::extract::Path((namespace, epoch)): axum::extract::Path<(String, String)>,
) -> Result<Json<SignatureResponse>, ApiError> {
    // Placeholder response
    Ok(Json(SignatureResponse {
        version: AuditVersion::default(),
        ciphersuite: Ciphersuite::default(),
        namespace,
        timestamp: 0,
        epoch: epoch.try_into().map_err(|_| ApiError::BadRequest("epoch is not an integer".to_string()))?,
        digest: vec![],
        signature: vec![],
        key_id: None,
        serialized_message: None,
    }))
}
