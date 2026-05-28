use std::vec;

use akd_watch_common::{
    storage::signing_keys::VerifyingKeyRepository, web_api::ServerConfiguration,
};
use axum::Json;
use tracing::{error, info, instrument};

use crate::AppState;

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
        .collect();
    Json(ServerConfiguration { keys })
}
