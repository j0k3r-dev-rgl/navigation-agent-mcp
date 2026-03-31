use navigation_engine::capabilities::inspect_tree::inspect_tree;
use navigation_engine::protocol::InspectTreeRequestPayload;
use tempfile::tempdir;

#[test]
fn hidden_files_respect_hard_ignores() {
    let workspace = tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join("src")).unwrap();
    std::fs::create_dir_all(workspace.path().join(".hidden")).unwrap();
    std::fs::create_dir_all(workspace.path().join(".git")).unwrap();
    std::fs::create_dir_all(workspace.path().join("node_modules")).unwrap();
    std::fs::write(workspace.path().join("src/main.py"), "ok\n").unwrap();
    std::fs::write(workspace.path().join(".hidden/note.txt"), "secret\n").unwrap();
    std::fs::write(workspace.path().join(".git/config"), "[core]\n").unwrap();

    let result = inspect_tree(
        workspace.path().to_string_lossy().as_ref(),
        InspectTreeRequestPayload {
            path: None,
            max_depth: 2,
            extensions: vec![],
            file_pattern: None,
            include_stats: false,
            include_hidden: true,
        },
    )
    .unwrap();

    let paths = result
        .items
        .iter()
        .map(|item| item.path.as_str())
        .collect::<Vec<_>>();
    assert!(paths.contains(&".hidden"));
    assert!(paths.contains(&".hidden/note.txt"));
    assert!(!paths
        .iter()
        .any(|path| *path == ".git" || path.starts_with(".git/")));
    assert!(!paths
        .iter()
        .any(|path| *path == "node_modules" || path.starts_with("node_modules/")));
}
