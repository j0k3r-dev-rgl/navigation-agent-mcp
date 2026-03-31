use navigation_engine::capabilities::trace_callers::trace_callers;
use navigation_engine::protocol::TraceCallersRequestPayload;
use tempfile::tempdir;

#[test]
fn traces_direct_and_recursive_typescript_callers() {
    let workspace = tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join("app/routes")).unwrap();
    std::fs::write(
        workspace.path().join("app/routes/dashboard.tsx"),
        "export async function loader() { return null; }\n",
    )
    .unwrap();
    std::fs::write(
        workspace.path().join("app/routes/layout.tsx"),
        "import { loader as dashboardLoader } from './dashboard';\nexport function Layout() { return dashboardLoader(); }\n",
    )
    .unwrap();
    std::fs::write(
        workspace.path().join("app/routes/root.tsx"),
        "export function Root() { return Layout(); }\n",
    )
    .unwrap();

    let result = trace_callers(
        workspace.path().to_string_lossy().as_ref(),
        TraceCallersRequestPayload {
            path: "app/routes/dashboard.tsx".to_string(),
            symbol: "loader".to_string(),
            analyzer_language: "typescript".to_string(),
            public_language_filter: Some("typescript".to_string()),
            recursive: true,
            max_depth: Some(3),
        },
    )
    .unwrap();

    assert_eq!(result.total_matched, 1);
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].path, "app/routes/layout.tsx");
    let recursive = result.recursive.expect("recursive payload");
    assert_eq!(recursive.classifications.direct_callers.len(), 1);
    assert!(recursive.max_depth_observed >= 1);
}

#[test]
fn traces_java_callers_and_classifies_probable_public_entry_points() {
    let workspace = tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join("src/main/java/com/example")).unwrap();
    std::fs::write(
        workspace.path().join("src/main/java/com/example/DashboardService.java"),
        "package com.example; public class DashboardService { public String loader() { return \"ok\"; } }",
    )
    .unwrap();
    std::fs::write(
        workspace.path().join("src/main/java/com/example/NavigationController.java"),
        "package com.example; @RestController public class NavigationController { @GetMapping(\"/dashboard\") public String getNavigation() { return loader(); } }",
    )
    .unwrap();

    let result = trace_callers(
        workspace.path().to_string_lossy().as_ref(),
        TraceCallersRequestPayload {
            path: "src/main/java/com/example/DashboardService.java".to_string(),
            symbol: "loader".to_string(),
            analyzer_language: "java".to_string(),
            public_language_filter: Some("java".to_string()),
            recursive: true,
            max_depth: Some(3),
        },
    )
    .unwrap();

    assert_eq!(result.items.len(), 1);
    let recursive = result.recursive.expect("recursive payload");
    assert_eq!(
        recursive.classifications.probable_public_entry_points.len(),
        1
    );
    assert_eq!(
        recursive.classifications.probable_public_entry_points[0].reasons,
        vec!["public controller method"]
    );
}
