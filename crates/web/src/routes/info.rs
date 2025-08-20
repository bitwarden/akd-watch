use std::vec;

use akd_watch_common::{crypto::VerifyingKey, storage::signing_keys::VerifyingKeyRepository};
use axum::Json;
use serde::{Deserialize, Serialize};
use tracing::{error, info, instrument};

use crate::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfiguration {
    keys: Vec<KeyInfo>,
    // Other configuration info
}

#[instrument(skip_all)]
pub async fn info_handler(
    axum::extract::State(AppState {
        verifying_key_storage,
        ..
    }): axum::extract::State<AppState>,
) -> Json<ServerConfiguration> {
    info!("Handling server info request");
    let keys = verifying_key_storage
        .list_keys()
        .await
        .unwrap_or_else(|e| {
            error!("Failed to list keys: {}", e);
            vec![]
        })
        .iter()
        .map(|key| key.into())
        .collect::<Vec<KeyInfo>>();
    Json(ServerConfiguration { keys })
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct KeyInfo {
    public_key: String,
    key_id: String,
    not_before: u64,
}

impl From<&VerifyingKey> for KeyInfo {
    fn from(key: &VerifyingKey) -> Self {
        Self {
            public_key: hex::encode(key.verifying_key),
            key_id: key.key_id.to_string(),
            not_before: key.not_before.timestamp() as u64,
        }
    }
}
