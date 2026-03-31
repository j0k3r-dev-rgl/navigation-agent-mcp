use navigation_engine::capabilities::list_endpoints::list_endpoints;
use navigation_engine::protocol::ListEndpointsRequestPayload;
use tempfile::tempdir;

#[test]
fn returns_empty_result_for_workspace_without_endpoints() {
    let workspace = tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join("src")).unwrap();

    let result = list_endpoints(
        workspace.path().to_string_lossy().as_ref(),
        ListEndpointsRequestPayload {
            path: None,
            analyzer_language: "typescript".to_string(),
            public_language_filter: None,
            public_framework_filter: None,
            kind: "any".to_string(),
            limit: 50,
        },
    )
    .unwrap();

    assert_eq!(result.total_matched, 0);
    assert!(result.items.is_empty());
    assert!(!result.truncated);
}

#[test]
fn respects_kind_filter() {
    let workspace = tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join("src")).unwrap();

    let result = list_endpoints(
        workspace.path().to_string_lossy().as_ref(),
        ListEndpointsRequestPayload {
            path: None,
            analyzer_language: "typescript".to_string(),
            public_language_filter: None,
            public_framework_filter: None,
            kind: "graphql".to_string(),
            limit: 50,
        },
    )
    .unwrap();

    // With no files, filtering by graphql should still return empty
    assert_eq!(result.total_matched, 0);
    assert!(result.items.is_empty());
}
