use navigation_engine::capabilities::search_text::search_text;
use navigation_engine::protocol::SearchTextRequestPayload;
use tempfile::tempdir;

#[test]
fn returns_search_matches_with_context_and_truncation() {
    if std::process::Command::new("rg")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }

    let workspace = tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join("src/routes")).unwrap();
    std::fs::write(
        workspace.path().join("src/routes/a.ts"),
        "const start = true;\nexport async function loader() {}\nreturn loader;\n",
    )
    .unwrap();
    std::fs::write(
        workspace.path().join("src/routes/b.ts"),
        "export const loaderState = true;\n",
    )
    .unwrap();

    let result = search_text(
        workspace.path().to_string_lossy().as_ref(),
        SearchTextRequestPayload {
            query: "loader".to_string(),
            path: Some("src".to_string()),
            public_language_filter: Some("typescript".to_string()),
            include: None,
            regex: false,
            context: 1,
            limit: 1,
        },
    )
    .unwrap();

    assert_eq!(result.resolved_path.as_deref(), Some("src"));
    assert_eq!(result.total_file_count, 2);
    assert_eq!(result.total_match_count, 3);
    assert!(result.truncated);
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].path, "src/routes/a.ts");
    assert_eq!(result.items[0].language.as_deref(), Some("typescript"));
    assert_eq!(result.items[0].match_count, 2);
    assert_eq!(result.items[0].matches[0].line, 2);
    assert_eq!(result.items[0].matches[0].before[0].line, 1);
    assert_eq!(result.items[0].matches[1].line, 3);
}

#[test]
fn returns_empty_results_for_ignored_scope() {
    let workspace = tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join("node_modules/demo")).unwrap();
    std::fs::write(
        workspace.path().join("node_modules/demo/index.ts"),
        "export const loader = true;\n",
    )
    .unwrap();

    let result = search_text(
        workspace.path().to_string_lossy().as_ref(),
        SearchTextRequestPayload {
            query: "loader".to_string(),
            path: Some("node_modules".to_string()),
            public_language_filter: Some("typescript".to_string()),
            include: None,
            regex: false,
            context: 1,
            limit: 10,
        },
    )
    .unwrap();

    assert_eq!(result.resolved_path.as_deref(), Some("node_modules"));
    assert_eq!(result.total_file_count, 0);
    assert_eq!(result.total_match_count, 0);
    assert!(result.items.is_empty());
    assert!(!result.truncated);
}
