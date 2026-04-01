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

#[test]
fn marks_recursive_calls() {
    let workspace = tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join("src/modules")).unwrap();
    // Java file with recursive function call
    std::fs::write(
        workspace.path().join("src/modules/Factorial.java"),
        r#"
package modules;
public class Factorial {
    public int factorial(int n) {
        if (n <= 1) return 1;
        return n * factorial(n - 1);  // Recursive call
    }
}
"#,
    )
    .unwrap();

    let result = trace_flow(
        workspace.path().to_string_lossy().as_ref(),
        TraceFlowRequestPayload {
            path: "src/modules/Factorial.java".to_string(),
            symbol: "factorial".to_string(),
            analyzer_language: "java".to_string(),
            public_language_filter: None,
            max_depth: None,
        },
    )
    .unwrap();

    // Find the recursive callee and check it's marked as recursive
    let recursive_callees: Vec<_> = result.callees.iter().filter(|c| c.recursive).collect();

    assert!(
        !recursive_callees.is_empty(),
        "expected at least one recursive callee, got {:?}",
        result.callees
    );

    // The recursive call should be to 'factorial'
    let has_recursive_factorial = recursive_callees.iter().any(|c| c.callee == "factorial");
    assert!(
        has_recursive_factorial,
        "expected recursive call to 'factorial', got {:?}",
        recursive_callees
    );
}

#[test]
fn detects_infrastructure_file_pattern() {
    // Test the is_infrastructure_file helper function directly
    use navigation_engine::capabilities::trace_flow::is_infrastructure_file;

    // Should match infrastructure/persistence paths
    assert!(is_infrastructure_file(
        "src/modules/user/infrastructure/persistence/UserAdapter.java"
    ));
    assert!(is_infrastructure_file(
        "src/modules/order/infrastructure/persistence/OrderRepository.java"
    ));
    assert!(is_infrastructure_file(
        "project/src/main/java/com/example/infrastructure/persistence/UserDAO.java"
    ));

    // Should match Adapter.java, Repository.java, DAO.java files (case insensitive)
    assert!(is_infrastructure_file("src/UserAdapter.java"));
    assert!(is_infrastructure_file("src/UserRepository.java"));
    assert!(is_infrastructure_file("src/UserDAO.java"));
    assert!(is_infrastructure_file("src/useradapter.java")); // lowercase

    // Should not match non-infrastructure files
    assert!(!is_infrastructure_file(
        "src/modules/user/application/UserService.java"
    ));
    assert!(!is_infrastructure_file("src/modules/user/domain/User.java"));
    assert!(!is_infrastructure_file("src/UserController.java"));
    assert!(!is_infrastructure_file("src/UserService.java"));
}

#[test]
fn callees_have_recursive_field_default() {
    let workspace = tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join("src/routes")).unwrap();
    std::fs::create_dir_all(workspace.path().join("src/lib")).unwrap();

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

    // All callees should have the recursive field (defaults to false for non-recursive)
    for callee in &result.callees {
        // The field exists and is a boolean - for non-recursive calls it should be false
        assert!(
            !callee.recursive,
            "Non-recursive callee '{}' should have recursive=false",
            callee.callee
        );
    }
}
