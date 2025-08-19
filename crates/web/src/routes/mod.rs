use axum::{Router, routing::get};

use crate::AppState;

mod audits;
mod info;
mod namespaces;

pub fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/info", get(info::info_handler))
        .route("/namespaces", get(namespaces::list_namespaces_handler))
        .route(
            "/namespaces/:namespace",
            get(namespaces::namespace_query_handler),
        )
        .route(
            "/namespaces/:namespace/audits/:epoch",
            get(audits::audit_query_handler),
        )
}
