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

#[test]
fn traces_rust_callers_for_functions_and_impl_methods() {
    let workspace = tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join("src")).unwrap();
    std::fs::write(
        workspace.path().join("src/lib.rs"),
        "fn load() {}\n\n#[get(\"/health\")]\nfn healthcheck() {\n    load();\n}\n\nstruct Builder;\n\nimpl Builder {\n    fn build() {\n        Self::reset();\n        let helper = Self::new();\n        helper.reset();\n    }\n\n    fn reset() {}\n    fn new() -> Self { Builder }\n}\n",
    )
    .unwrap();

    let function_result = trace_callers(
        workspace.path().to_string_lossy().as_ref(),
        TraceCallersRequestPayload {
            path: "src/lib.rs".to_string(),
            symbol: "load".to_string(),
            analyzer_language: "rust".to_string(),
            public_language_filter: Some("rust".to_string()),
            recursive: true,
            max_depth: Some(2),
        },
    )
    .unwrap();

    assert_eq!(function_result.items.len(), 1);
    assert_eq!(
        function_result.items[0].caller_symbol.as_deref(),
        Some("healthcheck")
    );
    let function_recursive = function_result.recursive.expect("recursive payload");
    assert_eq!(
        function_recursive
            .classifications
            .probable_public_entry_points[0]
            .reasons,
        vec!["public rest handler"]
    );

    let method_result = trace_callers(
        workspace.path().to_string_lossy().as_ref(),
        TraceCallersRequestPayload {
            path: "src/lib.rs".to_string(),
            symbol: "Builder::reset".to_string(),
            analyzer_language: "rust".to_string(),
            public_language_filter: Some("rust".to_string()),
            recursive: false,
            max_depth: None,
        },
    )
    .unwrap();

    assert_eq!(method_result.items.len(), 2);
    assert!(method_result
        .items
        .iter()
        .all(|item| item.caller_symbol.as_deref() == Some("Builder::build")));
}

#[test]
fn traces_direct_go_callers_through_field_backed_services() {
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
        "package handlers\n\nimport \"example/app/internal/service\"\n\ntype UserHandler struct { service *service.UserService }\n\nfunc (h *UserHandler) Handle() {\n    h.service.CreateUser()\n    h.service.CreateUser()\n}\n",
    )
    .unwrap();

    let result = trace_callers(
        workspace.path().to_string_lossy().as_ref(),
        TraceCallersRequestPayload {
            path: "internal/service/user_service.go".to_string(),
            symbol: "UserService.CreateUser".to_string(),
            analyzer_language: "go".to_string(),
            public_language_filter: Some("go".to_string()),
            recursive: false,
            max_depth: None,
        },
    )
    .unwrap();

    assert_eq!(result.items.len(), 1);
    assert_eq!(result.total_matched, 1);
    assert_eq!(
        result.items[0].path,
        "internal/http/handlers/user_handler.go"
    );
    assert_eq!(
        result.items[0].caller_symbol.as_deref(),
        Some("UserHandler.Handle")
    );
}

#[test]
fn traces_recursive_go_callers_end_to_end() {
    let workspace = tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join("internal/http/handlers")).unwrap();
    std::fs::create_dir_all(workspace.path().join("internal/http/routes")).unwrap();
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
        "package handlers\n\nimport \"example/app/internal/service\"\n\ntype UserHandler struct { service *service.UserService }\n\nfunc (h *UserHandler) Handle() {\n    h.service.CreateUser()\n}\n",
    )
    .unwrap();
    std::fs::write(
        workspace.path().join("internal/http/routes/user_routes.go"),
        "package routes\n\nimport \"example/app/internal/http/handlers\"\n\nfunc RegisterRoutes(handler *handlers.UserHandler) {\n    handler.Handle()\n}\n",
    )
    .unwrap();

    let result = trace_callers(
        workspace.path().to_string_lossy().as_ref(),
        TraceCallersRequestPayload {
            path: "internal/service/user_service.go".to_string(),
            symbol: "UserService.CreateUser".to_string(),
            analyzer_language: "go".to_string(),
            public_language_filter: Some("go".to_string()),
            recursive: true,
            max_depth: Some(3),
        },
    )
    .unwrap();

    assert_eq!(result.items.len(), 1);
    assert_eq!(
        result.items[0].caller_symbol.as_deref(),
        Some("UserHandler.Handle")
    );
    let recursive = result.recursive.expect("recursive payload");
    assert_eq!(recursive.classifications.direct_callers.len(), 1);
    assert_eq!(recursive.classifications.indirect_callers.len(), 1);
    assert_eq!(
        recursive.classifications.direct_callers[0].symbol,
        "UserHandler.Handle"
    );
    assert_eq!(
        recursive.classifications.indirect_callers[0].symbol,
        "RegisterRoutes"
    );
}

#[test]
fn traces_go_method_value_callers_from_handler_registration() {
    let workspace = tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join("cmd/api")).unwrap();
    std::fs::create_dir_all(workspace.path().join("internal/http/handlers")).unwrap();
    std::fs::create_dir_all(workspace.path().join("internal/service")).unwrap();
    std::fs::write(
        workspace.path().join("go.mod"),
        "module example/app\n\ngo 1.23.0\n",
    )
    .unwrap();
    std::fs::write(
        workspace.path().join("internal/service/user_service.go"),
        "package service\n\ntype UserService struct{}\n\nfunc (s *UserService) CreateUser() {}\n",
    )
    .unwrap();
    std::fs::write(
        workspace.path().join("internal/http/handlers/user_handler.go"),
        "package handlers\n\nimport \"example/app/internal/service\"\n\ntype UserHandler struct { service *service.UserService }\n\nfunc (h *UserHandler) CreateUser() {\n    h.service.CreateUser()\n}\n",
    )
    .unwrap();
    std::fs::write(
        workspace.path().join("cmd/api/main.go"),
        "package main\n\nimport (\n    \"net/http\"\n    \"example/app/internal/http/handlers\"\n)\n\nfunc main() {\n    handler := &handlers.UserHandler{}\n    mux := http.NewServeMux()\n    mux.HandleFunc(\"POST /users\", handler.CreateUser)\n}\n",
    )
    .unwrap();

    let result = trace_callers(
        workspace.path().to_string_lossy().as_ref(),
        TraceCallersRequestPayload {
            path: "internal/http/handlers/user_handler.go".to_string(),
            symbol: "UserHandler.CreateUser".to_string(),
            analyzer_language: "go".to_string(),
            public_language_filter: Some("go".to_string()),
            recursive: true,
            max_depth: Some(3),
        },
    )
    .unwrap();

    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].path, "cmd/api/main.go");
    assert_eq!(result.items[0].caller_symbol.as_deref(), Some("main"));
    assert_eq!(result.items[0].relation, "references");
}

#[test]
fn traces_go_interface_callers_to_concrete_repository_implementation() {
    let result = trace_callers(
        "/home/j0k3r/navigation-agent-mcp/examples/go",
        TraceCallersRequestPayload {
            path: "internal/repository/memory_user_repository.go".to_string(),
            symbol: "MemoryUserRepository.Save".to_string(),
            analyzer_language: "go".to_string(),
            public_language_filter: Some("go".to_string()),
            recursive: true,
            max_depth: Some(5),
        },
    )
    .unwrap();

    assert!(
        result
            .items
            .iter()
            .any(|item| item.caller_symbol.as_deref() == Some("UserService.persistUser")),
        "expected persistUser direct caller, got: {:?}",
        result.items
    );
    let recursive = result.recursive.expect("expected recursive result");
    assert!(
        recursive
            .classifications
            .indirect_callers
            .iter()
            .any(|item| item.symbol == "UserService.CreateUser"),
        "expected CreateUser indirect caller, got: {:?}",
        recursive.classifications.indirect_callers
    );
}
