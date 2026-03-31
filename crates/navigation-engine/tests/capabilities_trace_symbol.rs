use navigation_engine::capabilities::trace_symbol::trace_symbol;
use navigation_engine::protocol::TraceSymbolRequestPayload;
use tempfile::tempdir;

#[test]
fn traces_related_files_from_the_workspace_and_preserves_entrypoint_scope() {
    if std::process::Command::new("rg")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }

    let workspace = tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join("src/routes")).unwrap();
    std::fs::create_dir_all(workspace.path().join("src/shared")).unwrap();
    std::fs::write(
        workspace.path().join("src/routes/dashboard.tsx"),
        "export async function loader() {}\nexport function view() { return loader; }\n",
    )
    .unwrap();
    std::fs::write(
        workspace.path().join("src/shared/navigation.ts"),
        "export const current = loader;\n",
    )
    .unwrap();
    std::fs::write(
        workspace.path().join("src/shared/other.ts"),
        "export const nothing = true;\n",
    )
    .unwrap();

    let result = trace_symbol(
        workspace.path().to_string_lossy().as_ref(),
        TraceSymbolRequestPayload {
            path: "src/routes/dashboard.tsx".to_string(),
            symbol: "loader".to_string(),
            analyzer_language: "typescript".to_string(),
            public_language_filter: None,
        },
    )
    .unwrap();

    assert_eq!(
        result.resolved_path.as_deref(),
        Some("src/routes/dashboard.tsx")
    );
    assert_eq!(result.total_matched, 2);
    assert_eq!(result.items.len(), 2);
    assert_eq!(result.items[0].path, "src/routes/dashboard.tsx");
    assert_eq!(result.items[1].path, "src/shared/navigation.ts");
    assert!(!result.truncated);
}

#[test]
fn returns_symbol_not_found_when_the_entrypoint_does_not_define_the_symbol() {
    let workspace = tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join("src/routes")).unwrap();
    std::fs::write(
        workspace.path().join("src/routes/dashboard.tsx"),
        "export async function action() {}\n",
    )
    .unwrap();

    let error = trace_symbol(
        workspace.path().to_string_lossy().as_ref(),
        TraceSymbolRequestPayload {
            path: "src/routes/dashboard.tsx".to_string(),
            symbol: "loader".to_string(),
            analyzer_language: "typescript".to_string(),
            public_language_filter: None,
        },
    )
    .unwrap_err();

    assert_eq!(error.code, "SYMBOL_NOT_FOUND");
    assert_eq!(
        error.message,
        "Symbol 'loader' was not found in 'src/routes/dashboard.tsx'."
    );
}
