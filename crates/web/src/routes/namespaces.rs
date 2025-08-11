use akd_watch_common::{ akd_configurations::AkdConfiguration, Epoch, NamespaceInfo, NamespaceStatus};
use axum::Json;

pub async fn handle_query_namespace(
    axum::extract::Path(namespace): axum::extract::Path<String>,
) -> Json<Option<NamespaceInfo>> {
    // TODO: Placeholder Return list of blobs in the namespace
    Json(NamespaceInfo {
        configuration: AkdConfiguration::BitwardenV1Configuration,
        name: namespace.clone(),
        log_directory: format!("Namespace: {}", namespace),
        last_verified_epoch: Some(Epoch::new(42)),
        starting_epoch: Epoch::new(1),
        status: NamespaceStatus::Online,
    }.into())
}

pub async fn handle_list_namespaces() -> Json<Vec<String>> {
    // TODO: Placeholder Return list of namespaces
    Json(vec!["namespace1".to_string(), "namespace2".to_string()])
}
