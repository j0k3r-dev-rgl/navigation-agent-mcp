use std::collections::BTreeSet;

use navigation_engine::workspace::{
    canonicalize_workspace_root, collect_supported_source_files, resolve_scope,
};
use tempfile::tempdir;

#[test]
fn resolve_scope_returns_missing_path_error() {
    let workspace = tempdir().unwrap();
    let workspace_root =
        canonicalize_workspace_root(workspace.path().to_string_lossy().as_ref()).unwrap();

    let error = resolve_scope(&workspace_root, Some("missing")).unwrap_err();
    assert_eq!(error.code, "FILE_NOT_FOUND");
    assert_eq!(error.details, serde_json::json!({ "path": "missing" }));
}

#[test]
fn resolve_scope_rejects_paths_outside_workspace() {
    let workspace = tempdir().unwrap();
    let workspace_root =
        canonicalize_workspace_root(workspace.path().to_string_lossy().as_ref()).unwrap();
    let outside = workspace_root.parent().unwrap().join("outside.txt");

    let error =
        resolve_scope(&workspace_root, Some(outside.to_string_lossy().as_ref())).unwrap_err();
    assert_eq!(error.code, "PATH_OUTSIDE_WORKSPACE");
}

#[test]
fn collect_supported_source_files_skips_unsupported_inputs() {
    let workspace = tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join("docs")).unwrap();
    std::fs::write(workspace.path().join("docs/readme.md"), "# docs\n").unwrap();
    std::fs::write(workspace.path().join("notes.txt"), "notes\n").unwrap();

    let workspace_root =
        canonicalize_workspace_root(workspace.path().to_string_lossy().as_ref()).unwrap();
    let scope = resolve_scope(&workspace_root, None).unwrap();
    let supported_extensions = BTreeSet::from([
        ".java".to_string(),
        ".js".to_string(),
        ".jsx".to_string(),
        ".ts".to_string(),
        ".tsx".to_string(),
    ]);

    let files =
        collect_supported_source_files(&workspace_root, &scope, &supported_extensions, false)
            .unwrap();
    assert!(files.is_empty());
}
