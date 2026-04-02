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
    let root = result.root.expect("expected root node");
    let callee_names: Vec<&str> = root.callers.iter().map(|c| c.symbol.as_str()).collect();
    assert!(
        callee_names.contains(&"getData"),
        "expected 'getData' in callers, got {:?}",
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
    assert!(!result.truncated);
    let root = result.root.expect("expected root node");
    assert!(root.callers.is_empty());
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

    let root = result.root.expect("expected root node");
    let recursive_child = root
        .callers
        .iter()
        .find(|child| child.symbol.ends_with("factorial"))
        .expect("expected recursive factorial child");
    assert!(recursive_child.callers.is_empty());
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
fn tree_nodes_include_range_line_and_via() {
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

    let root = result.root.expect("expected root node");
    assert!(root.range_line.init > 0);
    let child = root.callers.iter().find(|c| c.symbol == "getData").unwrap();
    assert!(child.range_line.end >= child.range_line.init);
    assert!(child.via.as_ref().is_some_and(|via| !via.is_empty()));
}

#[test]
fn nests_java_interface_implementations_under_ports() {
    let workspace = tempdir().unwrap();
    std::fs::create_dir_all(
        workspace
            .path()
            .join("src/modules/sample/infrastructure/web"),
    )
    .unwrap();
    std::fs::create_dir_all(
        workspace
            .path()
            .join("src/modules/sample/application/ports/input"),
    )
    .unwrap();
    std::fs::create_dir_all(
        workspace
            .path()
            .join("src/modules/sample/application/ports/output"),
    )
    .unwrap();
    std::fs::create_dir_all(
        workspace
            .path()
            .join("src/modules/sample/application/use_cases/command"),
    )
    .unwrap();
    std::fs::create_dir_all(
        workspace
            .path()
            .join("src/modules/sample/infrastructure/persistence/dao"),
    )
    .unwrap();

    std::fs::write(
        workspace
            .path()
            .join("src/modules/sample/infrastructure/web/SampleController.java"),
        r#"
package sample.infrastructure.web;
import sample.application.ports.input.CreateThing;
public class SampleController {
    private final CreateThing createThingPort;
    public void create() {
        createThingPort.create();
    }
}
"#,
    )
    .unwrap();

    std::fs::write(
        workspace
            .path()
            .join("src/modules/sample/application/ports/input/CreateThing.java"),
        r#"
package sample.application.ports.input;
public interface CreateThing {
    void create();
}
"#,
    )
    .unwrap();

    std::fs::write(
        workspace
            .path()
            .join("src/modules/sample/application/ports/output/CreateThingRepository.java"),
        r#"
package sample.application.ports.output;
public interface CreateThingRepository {
    void save();
}
"#,
    )
    .unwrap();

    std::fs::write(
        workspace
            .path()
            .join("src/modules/sample/application/use_cases/command/CreateThingUseCase.java"),
        r#"
package sample.application.use_cases.command;
import sample.application.ports.input.CreateThing;
import sample.application.ports.output.CreateThingRepository;
public class CreateThingUseCase implements CreateThing {
    private final CreateThingRepository repository;
    public void create() {
        repository.save();
    }
}
"#,
    )
    .unwrap();

    std::fs::write(
        workspace
            .path()
            .join("src/modules/sample/infrastructure/persistence/dao/CreateThingAdapter.java"),
        r#"
package sample.infrastructure.persistence.dao;
import sample.application.ports.output.CreateThingRepository;
public class CreateThingAdapter implements CreateThingRepository {
    public void save() {
    }
}
"#,
    )
    .unwrap();

    let result = trace_flow(
        workspace.path().to_string_lossy().as_ref(),
        TraceFlowRequestPayload {
            path: "src/modules/sample/infrastructure/web/SampleController.java".to_string(),
            symbol: "create".to_string(),
            analyzer_language: "java".to_string(),
            public_language_filter: None,
            max_depth: Some(6),
        },
    )
    .unwrap();

    let root = result.root.expect("expected root node");
    let input_port = root
        .callers
        .iter()
        .find(|child| child.symbol.ends_with("CreateThing#create"))
        .expect("expected input port child");
    assert_eq!(input_port.kind, "interface-method");
    let use_case = input_port
        .callers
        .iter()
        .find(|child| child.symbol.ends_with("CreateThingUseCase#create"))
        .expect("expected use case nested under input port");
    assert_eq!(use_case.kind, "implementation-method");
    let output_port = use_case
        .callers
        .iter()
        .find(|child| child.symbol.ends_with("CreateThingRepository#save"))
        .expect("expected output port under use case");
    assert_eq!(output_port.kind, "interface-method");
    let adapter = output_port
        .callers
        .iter()
        .find(|child| child.symbol.ends_with("CreateThingAdapter#save"))
        .expect("expected adapter nested under output port");

    assert!(adapter.path.ends_with("CreateThingAdapter.java"));
    assert_eq!(adapter.kind, "implementation-method");
    assert_eq!(root.callers.first().map(|n| n.range_line.init), Some(4));
}
