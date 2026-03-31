use std::path::Path;

use navigation_engine::analyzers::language_analyzer::LanguageAnalyzer;
use navigation_engine::analyzers::typescript::TypeScriptAnalyzer;
use navigation_engine::analyzers::{FindCallersQuery, FindEndpointsQuery, FindSymbolQuery};

fn any_symbol_query() -> FindSymbolQuery {
    FindSymbolQuery {
        symbol: "loader".to_string(),
        kind: "any".to_string(),
        match_mode: "exact".to_string(),
        public_language_filter: None,
        limit: 50,
    }
}

fn any_endpoint_query() -> FindEndpointsQuery {
    FindEndpointsQuery {
        kind: "any".to_string(),
        public_language_filter: None,
        public_framework_filter: None,
        limit: 50,
    }
}

#[test]
fn extracts_typescript_definitions_with_public_kinds() {
    let analyzer = TypeScriptAnalyzer;
    let source = r#"
interface LoaderArgs { value: string }
type LoaderResult = string;
enum Mode { A, B }
class Worker {
  constructor() {}
  run() {}
}
function loader() {}
const action = () => {};
"#;

    let items = analyzer.find_symbols(
        Path::new("src/routes/example.ts"),
        source,
        &any_symbol_query(),
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

    assert!(kinds.contains(&("LoaderArgs", "interface", Some("typescript"))));
    assert!(kinds.contains(&("LoaderResult", "type", Some("typescript"))));
    assert!(kinds.contains(&("Mode", "enum", Some("typescript"))));
    assert!(kinds.contains(&("Worker", "class", Some("typescript"))));
    assert!(kinds.contains(&("constructor", "constructor", Some("typescript"))));
    assert!(kinds.contains(&("run", "method", Some("typescript"))));
    assert!(kinds.contains(&("loader", "function", Some("typescript"))));
    assert!(kinds.contains(&("action", "function", Some("typescript"))));
}

#[test]
fn extracts_javascript_definitions_with_javascript_language() {
    let analyzer = TypeScriptAnalyzer;
    let source = r#"
class Widget {
  render() {}
}
const loader = () => {};
function action() {}
"#;

    let items = analyzer.find_symbols(
        Path::new("src/routes/example.js"),
        source,
        &any_symbol_query(),
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

    assert!(kinds.contains(&("Widget", "class", Some("javascript"))));
    assert!(kinds.contains(&("render", "method", Some("javascript"))));
    assert!(kinds.contains(&("loader", "function", Some("javascript"))));
    assert!(kinds.contains(&("action", "function", Some("javascript"))));
}

#[test]
fn extracts_loader_function_as_endpoint_in_route_file() {
    let analyzer = TypeScriptAnalyzer;
    let source = r#"
export async function loader() {
  return json({ data: "test" });
}

export function action() {
  return redirect("/success");
}
"#;

    let items = analyzer.find_endpoints(
        Path::new("app/routes/dashboard.tsx"),
        source,
        &any_endpoint_query(),
    );
    assert_eq!(items.len(), 2);

    let loader = items
        .iter()
        .find(|e| e.name == "loader")
        .expect("loader endpoint");
    assert_eq!(loader.kind, "route");
    assert_eq!(loader.path, Some("/dashboard".to_string()));
    assert_eq!(loader.framework, Some("react-router".to_string()));
    assert_eq!(loader.language, Some("typescript".to_string()));
    assert_eq!(loader.line, 2);

    let action = items
        .iter()
        .find(|e| e.name == "action")
        .expect("action endpoint");
    assert_eq!(action.kind, "route");
    assert_eq!(action.path, Some("/dashboard".to_string()));
    assert_eq!(action.framework, Some("react-router".to_string()));
}

#[test]
fn extracts_arrow_function_loader_as_endpoint() {
    let analyzer = TypeScriptAnalyzer;
    let source = r#"
export const loader = async () => {
  return json({ data: "test" });
};
"#;

    let items = analyzer.find_endpoints(
        Path::new("app/routes/users.tsx"),
        source,
        &any_endpoint_query(),
    );
    assert_eq!(items.len(), 1);

    let loader = &items[0];
    assert_eq!(loader.name, "loader");
    assert_eq!(loader.kind, "route");
    assert_eq!(loader.path, Some("/users".to_string()));
}

#[test]
fn derives_dynamic_route_path_from_file() {
    let analyzer = TypeScriptAnalyzer;
    let source = r#"export function loader() { return json({}); }"#;

    let items = analyzer.find_endpoints(
        Path::new("app/routes/users.$id.tsx"),
        source,
        &any_endpoint_query(),
    );
    assert_eq!(items.len(), 1);

    let loader = &items[0];
    assert_eq!(loader.path, Some("/users/:id".to_string()));
}

#[test]
fn derives_index_route_path() {
    let analyzer = TypeScriptAnalyzer;
    let source = r#"export function loader() { return json({}); }"#;

    let items = analyzer.find_endpoints(
        Path::new("app/routes/_index.tsx"),
        source,
        &any_endpoint_query(),
    );
    assert_eq!(items.len(), 1);

    let loader = &items[0];
    assert_eq!(loader.path, Some("/".to_string()));
}

#[test]
fn ignores_non_route_files() {
    let analyzer = TypeScriptAnalyzer;
    let source = r#"
export function loader() {
  return json({});
}
"#;

    // Non-route file (not in app/routes/)
    let items = analyzer.find_endpoints(
        Path::new("src/utils/helper.ts"),
        source,
        &any_endpoint_query(),
    );
    assert_eq!(items.len(), 0);

    // Route file
    let items = analyzer.find_endpoints(
        Path::new("app/routes/dashboard.tsx"),
        source,
        &any_endpoint_query(),
    );
    assert_eq!(items.len(), 1);
}

#[test]
fn ignores_non_loader_action_functions_in_route_file() {
    let analyzer = TypeScriptAnalyzer;
    let source = r#"
export function loader() { return json({}); }
export function helper() { return "not an endpoint"; }
export const action = () => { return redirect("/"); };
"#;

    let items = analyzer.find_endpoints(
        Path::new("app/routes/test.tsx"),
        source,
        &any_endpoint_query(),
    );
    assert_eq!(items.len(), 2);

    let names: Vec<&str> = items.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"loader"));
    assert!(names.contains(&"action"));
    assert!(!names.contains(&"helper"));
}

#[test]
fn supports_react_router_framework_filter() {
    let analyzer = TypeScriptAnalyzer;
    assert!(analyzer.supports_framework(Some("react-router")));
    assert!(!analyzer.supports_framework(Some("spring")));
    assert!(analyzer.supports_framework(None));
}

#[test]
fn finds_typescript_callers_via_imported_aliases() {
    let analyzer = TypeScriptAnalyzer;
    let source = r#"
import { loader as dashboardLoader } from "./dashboard";

export function Layout() {
  return dashboardLoader();
}
"#;

    let items = analyzer.find_callers(
        Path::new("app/routes"),
        Path::new("app/routes/layout.tsx"),
        source,
        &FindCallersQuery {
            target_path: Path::new("app/routes/dashboard.tsx").to_path_buf(),
            target_symbol: "loader".to_string(),
        },
    );

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].caller, "Layout");
    assert_eq!(items[0].caller_symbol.as_deref(), Some("Layout"));
    assert_eq!(items[0].relation, "calls");
}
