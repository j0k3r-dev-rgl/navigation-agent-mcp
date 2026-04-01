use navigation_engine::capabilities::trace_flow::trace_flow;
use navigation_engine::protocol::TraceFlowRequestPayload;
use tempfile::tempdir;

#[test]
fn traces_callees_from_the_entrypoint_symbol() {
    let workspace = tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join("src/routes")).unwrap();
    std::fs::create_dir_all(workspace.path().join("src/lib")).unwrap();
    // loader calls getData which is defined in src/lib/data.ts
    std::fs::write(
        workspace.path().join("src/routes/dashboard.tsx"),
        "import { getData } from '../lib/data';\nexport async function loader() { return getData(); }\n",
    )
    .unwrap();
    std::fs::write(
        workspace.path().join("src/lib/data.ts"),
        "export function getData() { return []; }\n",
    )
    .unwrap();

    let result = trace_flow(
        workspace.path().to_string_lossy().as_ref(),
        TraceFlowRequestPayload {
            path: "src/routes/dashboard.tsx".to_string(),
            symbol: "loader".to_string(),
            analyzer_language: "typescript".to_string(),
            public_language_filter: None,
            max_depth: None,
        },
    )
    .unwrap();

    assert_eq!(
        result.resolved_path.as_deref(),
        Some("src/routes/dashboard.tsx")
    );
    assert!(!result.truncated);
    // trace_flow traces outgoing callees — loader calls getData, so at least 1 callee
    assert!(
        result.total_matched >= 1,
        "expected at least 1 callee, got {}",
        result.total_matched
    );
    assert_eq!(result.total_matched, result.callees.len());
    // The callee should be getData
    let callee_names: Vec<&str> = result.callees.iter().map(|c| c.callee.as_str()).collect();
    assert!(
        callee_names.contains(&"getData"),
        "expected 'getData' in callees, got {:?}",
        callee_names
    );
}

#[test]
fn returns_empty_result_when_the_symbol_has_no_callees() {
    let workspace = tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join("src/routes")).unwrap();
    // loader has an empty body — no callees
    std::fs::write(
        workspace.path().join("src/routes/dashboard.tsx"),
        "export async function loader() { return null; }\n",
    )
    .unwrap();

    let result = trace_flow(
        workspace.path().to_string_lossy().as_ref(),
        TraceFlowRequestPayload {
            path: "src/routes/dashboard.tsx".to_string(),
            symbol: "loader".to_string(),
            analyzer_language: "typescript".to_string(),
            public_language_filter: None,
            max_depth: None,
        },
    )
    .unwrap();

    assert_eq!(
        result.resolved_path.as_deref(),
        Some("src/routes/dashboard.tsx")
    );
    assert_eq!(result.total_matched, 0);
    assert!(result.callees.is_empty());
    assert!(!result.truncated);
}
