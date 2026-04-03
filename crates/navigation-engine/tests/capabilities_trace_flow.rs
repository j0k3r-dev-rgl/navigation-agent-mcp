use navigation_engine::capabilities::trace_flow::trace_flow;
use navigation_engine::protocol::TraceFlowRequestPayload;
use tempfile::tempdir;

#[test]
fn traces_callees_from_the_entrypoint_symbol() {
    let workspace = tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join("src/routes")).unwrap();
    std::fs::create_dir_all(workspace.path().join("src/lib")).unwrap();
    std::fs::create_dir_all(workspace.path().join("src/utils")).unwrap();
    // loader calls getData which is defined in src/lib/data.ts
    std::fs::write(
        workspace.path().join("src/routes/dashboard.tsx"),
        "import { getData } from '../lib/data';\nexport async function loader() { return getData(); }\n",
    )
    .unwrap();
    std::fs::write(
        workspace.path().join("src/lib/data.ts"),
        "import { formatData } from '../utils/format';\nexport function getData() { return formatData([]); }\n",
    )
    .unwrap();
    std::fs::write(
        workspace.path().join("src/utils/format.ts"),
        "export function formatData(input: unknown[]) { return input; }\n",
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
    let root = result.root.clone().expect("expected root node");
    let get_data = root
        .callers
        .iter()
        .find(|c| c.symbol == "getData")
        .expect("expected getData child");
    assert_eq!(get_data.path, "src/lib/data.ts");
    let format_data = get_data
        .callers
        .iter()
        .find(|c| c.symbol == "formatData")
        .expect("expected nested formatData child");
    assert_eq!(format_data.path, "src/utils/format.ts");
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
    let root = result.root.clone().expect("expected root node");
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

    let root = result.root.clone().expect("expected root node");
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
fn traces_javascript_imported_functions() {
    let workspace = tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join("src/routes")).unwrap();
    std::fs::create_dir_all(workspace.path().join("src/lib")).unwrap();

    std::fs::write(
        workspace.path().join("src/routes/page.js"),
        "import { submitForm } from '../lib/actions.js';\nexport async function action() { return submitForm(); }\n",
    )
    .unwrap();
    std::fs::write(
        workspace.path().join("src/lib/actions.js"),
        "export function submitForm() { return ok(); }\nfunction ok() { return true; }\n",
    )
    .unwrap();

    let result = trace_flow(
        workspace.path().to_string_lossy().as_ref(),
        TraceFlowRequestPayload {
            path: "src/routes/page.js".to_string(),
            symbol: "action".to_string(),
            analyzer_language: "typescript".to_string(),
            public_language_filter: None,
            max_depth: None,
        },
    )
    .unwrap();

    let root = result.root.expect("expected root node");
    let submit_form = root
        .callers
        .iter()
        .find(|c| c.symbol == "submitForm")
        .expect("expected submitForm child");
    assert_eq!(submit_form.path, "src/lib/actions.js");
    let ok = submit_form
        .callers
        .iter()
        .find(|c| c.symbol == "ok")
        .expect("expected nested ok child");
    assert_eq!(ok.path, "src/lib/actions.js");
}

#[test]
fn traces_exported_destructured_symbols_to_their_module() {
    let workspace = tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join("src/routes")).unwrap();
    std::fs::create_dir_all(workspace.path().join("src/services")).unwrap();

    std::fs::write(
        workspace.path().join("src/routes/page.tsx"),
        "import { getSession, commitSession } from '../services/cookies.service.server';\nexport async function action(request: Request) {\n  const session = await getSession(request.headers.get('Cookie'));\n  return { headers: { 'Set-Cookie': await commitSession(session) } };\n}\n",
    )
    .unwrap();
    std::fs::write(
        workspace.path().join("src/services/cookies.service.server.ts"),
        "const authSessionStorage = createCookieSessionStorage({});\nexport const { getSession, commitSession, destroySession } = authSessionStorage;\n",
    )
    .unwrap();

    let result = trace_flow(
        workspace.path().to_string_lossy().as_ref(),
        TraceFlowRequestPayload {
            path: "src/routes/page.tsx".to_string(),
            symbol: "action".to_string(),
            analyzer_language: "typescript".to_string(),
            public_language_filter: None,
            max_depth: None,
        },
    )
    .unwrap();

    let root = result.root.clone().expect("expected root node");
    let get_session = root
        .callers
        .iter()
        .find(|c| c.symbol == "getSession")
        .expect("expected getSession child");
    assert_eq!(get_session.path, "src/services/cookies.service.server.ts");
    let rendered = serde_json::to_string_pretty(&result).unwrap();
    assert!(rendered.contains("getSession"), "tree was: {rendered}");
}

#[test]
fn traces_go_method_flow_from_simple_method_name_across_example_layers() {
    let workspace = tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join("internal/http/handlers")).unwrap();
    std::fs::create_dir_all(workspace.path().join("internal/service")).unwrap();
    std::fs::create_dir_all(workspace.path().join("internal/domain")).unwrap();
    std::fs::create_dir_all(workspace.path().join("internal/repository")).unwrap();
    std::fs::write(
        workspace.path().join("go.mod"),
        "module example/app\n\ngo 1.23.0\n",
    )
    .unwrap();
    std::fs::write(
        workspace.path().join("internal/domain/user.go"),
        "package domain\n\nfunc NewUser() User { return User{} }\n\ntype User struct{}\n",
    )
    .unwrap();
    std::fs::write(
        workspace.path().join("internal/repository/user_repository.go"),
        "package repository\n\nimport \"example/app/internal/domain\"\n\ntype UserRepository interface { Save(domain.User) domain.User }\n",
    )
    .unwrap();
    std::fs::write(
        workspace.path().join("internal/service/user_service.go"),
        "package service\n\nimport (\n  \"example/app/internal/domain\"\n  \"example/app/internal/repository\"\n)\n\ntype UserService struct { repository repository.UserRepository }\n\nfunc (s *UserService) CreateUser() domain.User {\n  user := domain.NewUser()\n  return s.repository.Save(user)\n}\n",
    )
    .unwrap();
    std::fs::write(
        workspace.path().join("internal/http/handlers/user_handler.go"),
        "package handlers\n\nimport \"example/app/internal/service\"\n\ntype UserHandler struct { service *service.UserService }\n\nfunc writeJSON() {}\n\nfunc (h *UserHandler) CreateUser() {\n  h.service.CreateUser()\n  writeJSON()\n}\n",
    )
    .unwrap();

    let result = trace_flow(
        workspace.path().to_string_lossy().as_ref(),
        TraceFlowRequestPayload {
            path: "internal/http/handlers/user_handler.go".to_string(),
            symbol: "CreateUser".to_string(),
            analyzer_language: "go".to_string(),
            public_language_filter: Some("go".to_string()),
            max_depth: None,
        },
    )
    .unwrap();

    let root = result.root.expect("expected root node");
    let service_create_user = root
        .callers
        .iter()
        .find(|c| c.symbol == "UserService.CreateUser")
        .expect("expected service method child");
    assert_eq!(service_create_user.path, "internal/service/user_service.go");
    assert!(
        service_create_user
            .callers
            .iter()
            .any(|c| c.symbol == "NewUser"),
        "expected domain.NewUser nested callee"
    );
    assert!(root.callers.iter().any(|c| c.symbol == "writeJSON"));
}

#[test]
fn traces_nested_call_expressions_at_any_depth() {
    let workspace = tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join("src/routes")).unwrap();
    std::fs::create_dir_all(workspace.path().join("src/lib")).unwrap();

    std::fs::write(
        workspace.path().join("src/routes/page.tsx"),
        "import { first } from '../lib/first';\nimport { second } from '../lib/second';\nexport async function action() { return wrap({ nested: await first(second()) }); }\nfunction wrap(input: unknown) { return input; }\n",
    )
    .unwrap();
    std::fs::write(
        workspace.path().join("src/lib/first.ts"),
        "export async function first(value: unknown) { return value; }\n",
    )
    .unwrap();
    std::fs::write(
        workspace.path().join("src/lib/second.ts"),
        "export function second() { return 1; }\n",
    )
    .unwrap();

    let result = trace_flow(
        workspace.path().to_string_lossy().as_ref(),
        TraceFlowRequestPayload {
            path: "src/routes/page.tsx".to_string(),
            symbol: "action".to_string(),
            analyzer_language: "typescript".to_string(),
            public_language_filter: None,
            max_depth: None,
        },
    )
    .unwrap();

    let root = result.root.expect("expected root node");
    let names: Vec<_> = root.callers.iter().map(|n| n.symbol.as_str()).collect();
    assert!(names.contains(&"wrap"), "names were: {names:?}");
    assert!(names.contains(&"first"), "names were: {names:?}");
    assert!(names.contains(&"second"), "names were: {names:?}");
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

#[test]
fn traces_rust_impl_methods_with_qualified_symbol() {
    let workspace = tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join("src")).unwrap();
    std::fs::write(
        workspace.path().join("src/lib.rs"),
        "struct Builder;\nimpl Builder {\n    fn build() {\n        Self::new_empty();\n        Self::is_empty();\n    }\n\n    fn new_empty() {}\n    fn is_empty() {}\n}\n",
    )
    .unwrap();

    let result = trace_flow(
        workspace.path().to_string_lossy().as_ref(),
        TraceFlowRequestPayload {
            path: "src/lib.rs".to_string(),
            symbol: "Builder::build".to_string(),
            analyzer_language: "rust".to_string(),
            public_language_filter: None,
            max_depth: None,
        },
    )
    .unwrap();

    let root = result.root.expect("expected root node");
    let names: Vec<_> = root.callers.iter().map(|n| n.symbol.as_str()).collect();
    assert!(
        names.contains(&"Builder::new_empty"),
        "names were: {names:?}"
    );
    assert!(
        names.contains(&"Builder::is_empty"),
        "names were: {names:?}"
    );
}

#[test]
fn traces_go_handler_flow_across_service_calls() {
    let workspace = tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join("internal/http/handlers")).unwrap();
    std::fs::create_dir_all(workspace.path().join("internal/service")).unwrap();
    std::fs::write(
        workspace.path().join("go.mod"),
        "module example/app\n\ngo 1.23.0\n",
    )
    .unwrap();
    std::fs::write(
        workspace.path().join("internal/service/user_service.go"),
        "package service\n\ntype UserService struct {}\n\nfunc (s *UserService) CreateUser() {}\n",
    )
    .unwrap();
    std::fs::write(
        workspace.path().join("internal/http/handlers/user_handler.go"),
        "package handlers\n\nimport \"example/app/internal/service\"\n\ntype UserHandler struct { service *service.UserService }\n\nfunc (h *UserHandler) CreateUser() {\n    h.service.CreateUser()\n    writeJSON()\n}\n\nfunc writeJSON() {}\n",
    )
    .unwrap();

    let result = trace_flow(
        workspace.path().to_string_lossy().as_ref(),
        TraceFlowRequestPayload {
            path: "internal/http/handlers/user_handler.go".to_string(),
            symbol: "UserHandler.CreateUser".to_string(),
            analyzer_language: "go".to_string(),
            public_language_filter: Some("go".to_string()),
            max_depth: None,
        },
    )
    .unwrap();

    let root = result.root.expect("expected root node");
    let names: Vec<_> = root.callers.iter().map(|n| n.symbol.as_str()).collect();
    assert!(
        names.contains(&"UserService.CreateUser"),
        "names were: {names:?}"
    );
    assert!(names.contains(&"writeJSON"), "names were: {names:?}");
}

#[test]
fn traces_rust_instance_method_calls_from_local_bindings() {
    let workspace = tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join("src")).unwrap();
    std::fs::write(
        workspace.path().join("src/lib.rs"),
        "struct Builder;\nimpl Builder {\n    fn build() {\n        let index = Self::new_empty();\n        index.scan_project();\n    }\n\n    fn new_empty() -> Self { Builder }\n    fn scan_project(&self) {}\n}\n",
    )
    .unwrap();

    let result = trace_flow(
        workspace.path().to_string_lossy().as_ref(),
        TraceFlowRequestPayload {
            path: "src/lib.rs".to_string(),
            symbol: "Builder::build".to_string(),
            analyzer_language: "rust".to_string(),
            public_language_filter: None,
            max_depth: None,
        },
    )
    .unwrap();

    let root = result.root.expect("expected root node");
    let names: Vec<_> = root.callers.iter().map(|n| n.symbol.as_str()).collect();
    assert!(
        names.contains(&"Builder::new_empty"),
        "names were: {names:?}"
    );
    assert!(
        names.contains(&"Builder::scan_project"),
        "names were: {names:?}"
    );
}

#[test]
fn traces_rust_instance_method_calls_from_explicit_type_factory_bindings() {
    let workspace = tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join("src")).unwrap();
    std::fs::write(
        workspace.path().join("src/lib.rs"),
        "struct JavaProjectIndex;\nimpl JavaProjectIndex {\n    fn new_empty() -> Self { JavaProjectIndex }\n    fn scan_project(&self) {}\n}\n\nstruct Builder;\nimpl Builder {\n    fn build() {\n        let index = JavaProjectIndex::new_empty();\n        index.scan_project();\n    }\n}\n",
    )
    .unwrap();

    let result = trace_flow(
        workspace.path().to_string_lossy().as_ref(),
        TraceFlowRequestPayload {
            path: "src/lib.rs".to_string(),
            symbol: "Builder::build".to_string(),
            analyzer_language: "rust".to_string(),
            public_language_filter: None,
            max_depth: None,
        },
    )
    .unwrap();

    let root = result.root.expect("expected root node");
    let names: Vec<_> = root.callers.iter().map(|n| n.symbol.as_str()).collect();
    assert!(
        names.contains(&"JavaProjectIndex::new_empty"),
        "names were: {names:?}"
    );
    assert!(
        names.contains(&"JavaProjectIndex::scan_project"),
        "names were: {names:?}"
    );
}
