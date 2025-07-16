use axum::{routing::get, Router};

mod audits;
mod info;
mod namespaces;

pub fn api_routes() -> Router {
    Router::new()
        .route("/info", get(info::handle_info))
        .route("/namespaces", get(namespaces::handle_list_namespaces))
        .route(
            "/namespaces/:namespace",
            get(namespaces::handle_query_namespace),
        )
        .route(
            "/namespaces/:namespace/audits/:epoch",
            get(audits::handle_audit_query),
        )
}
