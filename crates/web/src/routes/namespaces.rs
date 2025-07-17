use akd_watch_common::{ configurations::AkdConfiguration, AuditVersion, Epoch, NamespaceInfo, NamespaceStatus};
use axum::Json;

pub async fn handle_query_namespace(
    axum::extract::Path(namespace): axum::extract::Path<String>,
) -> Json<Option<NamespaceInfo>> {
    // Placeholder: Return list of blobs in the namespace
    Json(NamespaceInfo {
        configuration: AkdConfiguration::BitwardenV1Configuration,
        name: namespace.clone(),
        log_directory: Some(format!("Namespace: {}", namespace)),
        last_verified_epoch: Some(Epoch::new(42)),
        status: NamespaceStatus::Online,
        signature_version: AuditVersion::default(),
    }.into())
}

pub async fn handle_list_namespaces() -> Json<Vec<String>> {
    // Placeholder: Return list of namespaces
    Json(vec!["namespace1".to_string(), "namespace2".to_string()])
}
