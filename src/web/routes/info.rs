use std::vec;

use axum::Json;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfiguration {
    keys: Vec<KeyInfo>,
    // Other configuration info
}

pub async fn handle_info() -> Json<ServerConfiguration> {
    Json(ServerConfiguration {
        keys: vec![],
    }.into())
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct KeyInfo {
    public_key: String,
    not_before: u64,
}
