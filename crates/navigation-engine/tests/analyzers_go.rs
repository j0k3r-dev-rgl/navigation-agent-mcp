use std::path::Path;

use navigation_engine::analyzers::go::GoAnalyzer;
use navigation_engine::analyzers::language_analyzer::LanguageAnalyzer;
use navigation_engine::analyzers::types::FindCalleesQuery;
use navigation_engine::analyzers::{FindCallersQuery, FindSymbolQuery};
use tempfile::tempdir;

fn any_query() -> FindSymbolQuery {
    FindSymbolQuery {
        symbol: "".to_string(),
        kind: "any".to_string(),
        match_mode: "fuzzy".to_string(),
        public_language_filter: None,
        limit: 50,
    }
}

fn go_callers_query(path: &str, symbol: &str) -> FindCallersQuery {
    FindCallersQuery {
        target_path: Path::new(path).to_path_buf(),
        target_symbol: symbol.to_string(),
    }
}

#[test]
fn extracts_go_definitions_with_public_kinds() {
    let analyzer = GoAnalyzer;
    let source = r#"
type User struct {}
type UserRepository interface { Save() error }
type Result = string

func ListUsers() {}

type UserService struct {}

func (s *UserService) CreateUser() {}
"#;

    let items = analyzer.find_symbols(
        Path::new("internal/service/user_service.go"),
        source,
        &any_query(),
    );
    let kinds = items
        .iter()
        .map(|item| {
            (
                item.symbol.as_str(),
                item.kind.as_str(),
                item.language.as_deref(),
            )
        })
        .collect::<Vec<_>>();

    assert!(kinds.contains(&("User", "class", Some("go"))));
    assert!(kinds.contains(&("UserRepository", "interface", Some("go"))));
    assert!(kinds.contains(&("Result", "type", Some("go"))));
    assert!(kinds.contains(&("ListUsers", "function", Some("go"))));
    assert!(kinds.contains(&("UserService.CreateUser", "method", Some("go"))));
}

#[test]
fn extracts_go_callees_across_fields_and_functions() {
    let analyzer = GoAnalyzer;
    let source = r#"
package handlers

type UserService struct {}
func (s *UserService) CreateUser() {}

type UserHandler struct { service *UserService }

func (h *UserHandler) Handle() {
    h.service.CreateUser()
    writeJSON()
}

func writeJSON() {}
"#;

    let callees = analyzer.find_callees(
        Path::new("internal/http/handlers/user_handler.go"),
        source,
        &FindCalleesQuery {
            target_symbol: "UserHandler.Handle".to_string(),
        },
    );

    let names = callees
        .iter()
        .map(|item| item.callee.as_str())
        .collect::<Vec<_>>();
    assert!(
        names.contains(&"UserService.CreateUser"),
        "names were: {names:?}"
    );
    assert!(names.contains(&"writeJSON"), "names were: {names:?}");
}

#[test]
fn traces_go_method_callees_when_target_uses_simple_method_name() {
    let analyzer = GoAnalyzer;
    let source = r#"
package handlers

import "example/app/internal/service"

type UserHandler struct { service *service.UserService }

func writeJSON() {}

func (h *UserHandler) CreateUser() {
    h.service.CreateUser()
    writeJSON()
}
"#;

    let callees = analyzer.find_callees(
        Path::new("internal/http/handlers/user_handler.go"),
        source,
        &FindCalleesQuery {
            target_symbol: "CreateUser".to_string(),
        },
    );

    let names = callees
        .iter()
        .map(|item| item.callee.as_str())
        .collect::<Vec<_>>();
    assert!(
        names.contains(&"UserService.CreateUser"),
        "names were: {names:?}"
    );
    assert!(names.contains(&"writeJSON"), "names were: {names:?}");
}

#[test]
fn finds_go_methods_by_simple_name_in_exact_mode() {
    let analyzer = GoAnalyzer;
    let source = r#"
package handlers

type UserHandler struct {}

func (h *UserHandler) CreateUser() {}
"#;

    let items = analyzer.find_symbols(
        Path::new("internal/http/handlers/user_handler.go"),
        source,
        &FindSymbolQuery {
            symbol: "CreateUser".to_string(),
            kind: "method".to_string(),
            match_mode: "exact".to_string(),
            public_language_filter: Some("go".to_string()),
            limit: 10,
        },
    );

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].symbol, "UserHandler.CreateUser");
}

#[test]
fn traces_go_import_and_field_backed_callees_inside_service_method() {
    let analyzer = GoAnalyzer;
    let workspace = tempdir().unwrap();
    let domain_dir = workspace.path().join("internal/domain");
    let repository_dir = workspace.path().join("internal/repository");
    let service_dir = workspace.path().join("internal/service");
    std::fs::create_dir_all(&domain_dir).unwrap();
    std::fs::create_dir_all(&repository_dir).unwrap();
    std::fs::create_dir_all(&service_dir).unwrap();
    std::fs::write(
        workspace.path().join("go.mod"),
        "module examples/goapp\n\ngo 1.23.0\n",
    )
    .unwrap();
    std::fs::write(
        domain_dir.join("user.go"),
        "package domain\n\n type User struct{}\n\n func NewUser(id string, name string, email string) User { return User{} }\n",
    )
    .unwrap();
    std::fs::write(
        repository_dir.join("user_repository.go"),
        "package repository\n\n import \"examples/goapp/internal/domain\"\n\n type UserRepository interface { Save(user domain.User) domain.User }\n",
    )
    .unwrap();
    let service_path = service_dir.join("user_service.go");
    std::fs::write(
        &service_path,
        "package service\n\n import (\n   \"examples/goapp/internal/domain\"\n   \"examples/goapp/internal/repository\"\n )\n\n type UserService struct { repository repository.UserRepository }\n\n func (s *UserService) CreateUser(name string, email string) (domain.User, error) {\n   user := domain.NewUser(\"1\", name, email)\n   return s.repository.Save(user), nil\n }\n",
    )
    .unwrap();
    let source = std::fs::read_to_string(&service_path).unwrap();

    let callees = analyzer.find_callees(
        &service_path,
        &source,
        &FindCalleesQuery {
            target_symbol: "UserService.CreateUser".to_string(),
        },
    );

    let names = callees
        .iter()
        .map(|item| item.callee.as_str())
        .collect::<Vec<_>>();
    assert!(names.contains(&"NewUser"), "names were: {names:?}");
    assert!(
        names.contains(&"UserRepository.Save"),
        "names were: {names:?}"
    );
}

#[test]
fn finds_same_file_go_function_callers() {
    let analyzer = GoAnalyzer;
    let source = r#"
package handlers

func CreateUser() {}

func Handle() {
    CreateUser()
}
"#;

    let items = analyzer.find_callers(
        Path::new("internal/http/handlers/user_handler.go"),
        Path::new("internal/http/handlers/user_handler.go"),
        source,
        &go_callers_query("internal/http/handlers/user_handler.go", "CreateUser"),
    );

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].caller_symbol.as_deref(), Some("Handle"));
    assert_eq!(items[0].calls.symbol, "CreateUser");
}

#[test]
fn finds_imported_and_field_backed_go_callers() {
    let analyzer = GoAnalyzer;
    let workspace = tempdir().unwrap();
    let handlers_dir = workspace.path().join("internal/http/handlers");
    let service_dir = workspace.path().join("internal/service");
    std::fs::create_dir_all(&handlers_dir).unwrap();
    std::fs::create_dir_all(&service_dir).unwrap();
    std::fs::write(
        workspace.path().join("go.mod"),
        "module example/app\n\ngo 1.23.0\n",
    )
    .unwrap();
    let service_path = service_dir.join("user_service.go");
    std::fs::write(
        &service_path,
        "package service\n\nfunc SaveUser() {}\n\ntype UserService struct {}\n\nfunc (s *UserService) CreateUser() {}\n",
    )
    .unwrap();
    let handler_path = handlers_dir.join("user_handler.go");
    std::fs::write(
        &handler_path,
        "package handlers\n\nimport \"example/app/internal/service\"\n\nfunc Route() {\n    service.SaveUser()\n}\n\ntype UserHandler struct { service *service.UserService }\n\nfunc (h *UserHandler) Handle() {\n    h.service.CreateUser()\n}\n",
    )
    .unwrap();
    let handler_source = std::fs::read_to_string(&handler_path).unwrap();

    let function_items = analyzer.find_callers(
        workspace.path(),
        &handler_path,
        &handler_source,
        &FindCallersQuery {
            target_path: service_path.clone(),
            target_symbol: "SaveUser".to_string(),
        },
    );
    assert_eq!(function_items.len(), 1);
    assert_eq!(function_items[0].caller_symbol.as_deref(), Some("Route"));

    let method_items = analyzer.find_callers(
        workspace.path(),
        &handler_path,
        &handler_source,
        &FindCallersQuery {
            target_path: service_path,
            target_symbol: "UserService.CreateUser".to_string(),
        },
    );
    assert_eq!(method_items.len(), 1);
    assert_eq!(
        method_items[0].caller_symbol.as_deref(),
        Some("UserHandler.Handle")
    );
    assert_eq!(method_items[0].calls.symbol, "UserService.CreateUser");
}

#[test]
fn keeps_go_method_matching_receiver_aware_and_conservative() {
    let analyzer = GoAnalyzer;
    let source = r#"
package service

type UserService struct {}
type AuditService struct {}

func (s *UserService) CreateUser() {}
func (s *AuditService) CreateUser() {}

func (s *UserService) Seed() {
    s.CreateUser()
}

func Register(handler *UserService, audit *AuditService) {
    handler.CreateUser()
    _ = audit.CreateUser
}
"#;

    let items = analyzer.find_callers(
        Path::new("internal/service/user_service.go"),
        Path::new("internal/service/user_service.go"),
        source,
        &go_callers_query("internal/service/user_service.go", "UserService.CreateUser"),
    );

    let callers = items
        .iter()
        .map(|item| item.caller_symbol.as_deref().unwrap_or_default())
        .collect::<Vec<_>>();
    assert_eq!(callers.len(), 2);
    assert!(callers.contains(&"UserService.Seed"));
    assert!(callers.contains(&"Register"));
}
