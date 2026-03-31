use std::path::Path;

use navigation_engine::analyzers::language_analyzer::LanguageAnalyzer;
use navigation_engine::analyzers::rust::RustAnalyzer;
use navigation_engine::analyzers::{FindEndpointsQuery, FindSymbolQuery};

fn any_query() -> FindSymbolQuery {
    FindSymbolQuery {
        symbol: "load".to_string(),
        kind: "any".to_string(),
        match_mode: "fuzzy".to_string(),
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

fn rest_endpoint_query() -> FindEndpointsQuery {
    FindEndpointsQuery {
        kind: "rest".to_string(),
        public_language_filter: None,
        public_framework_filter: None,
        limit: 50,
    }
}

fn graphql_endpoint_query() -> FindEndpointsQuery {
    FindEndpointsQuery {
        kind: "graphql".to_string(),
        public_language_filter: None,
        public_framework_filter: None,
        limit: 50,
    }
}

#[test]
fn extracts_supported_rust_definitions() {
    let analyzer = RustAnalyzer;
    let source = r#"
pub struct UserId;
pub enum JobState { Ready }
pub trait Runner {
    fn run(&self);
}
pub type LoadResult = String;
pub fn load() {}

impl UserId {
    pub fn new() -> Self { UserId }

    #[cfg(test)]
    pub fn load_cached(&self) {}
}
"#;

    let items = analyzer.find_symbols(Path::new("src/lib.rs"), source, &any_query());
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

    assert!(kinds.contains(&("UserId", "type", Some("rust"))));
    assert!(kinds.contains(&("JobState", "enum", Some("rust"))));
    assert!(kinds.contains(&("Runner", "interface", Some("rust"))));
    assert!(kinds.contains(&("LoadResult", "type", Some("rust"))));
    assert!(kinds.contains(&("load", "function", Some("rust"))));
    assert!(kinds.contains(&("new", "method", Some("rust"))));
    assert!(kinds.contains(&("load_cached", "method", Some("rust"))));
    assert_eq!(
        items
            .iter()
            .filter(|item| item.symbol == "load_cached")
            .count(),
        1
    );
}

#[test]
fn excludes_unsupported_rust_constructs() {
    let analyzer = RustAnalyzer;
    let source = r#"
macro_rules! nope { () => {} }
mod nested {}
union Bits { value: u32 }
const LIMIT: u32 = 1;
static NAME: &str = "x";
extern "C" { fn ffi(); }

trait Shape {
    fn area(&self);
    const SIDES: usize;
    type Output;
}

fn outer() {
    fn inner() {}
}
"#;

    let items = analyzer.find_symbols(Path::new("src/unsupported.rs"), source, &any_query());
    let names = items
        .iter()
        .map(|item| item.symbol.as_str())
        .collect::<Vec<_>>();

    assert_eq!(names, vec!["Shape", "outer"]);
    assert!(!names.contains(&"ffi"));
    assert!(!names.contains(&"area"));
    assert!(!names.contains(&"SIDES"));
    assert!(!names.contains(&"Output"));
    assert!(!names.contains(&"inner"));
}

#[test]
fn extracts_actix_endpoint_with_full_path_attribute() {
    let analyzer = RustAnalyzer;
    let source = r#"
use actix_web::Responder;

#[actix_web::get("/")]
async fn index() -> impl Responder {
    "Hello World"
}
"#;

    let items = analyzer.find_endpoints(Path::new("src/main.rs"), source, &any_endpoint_query());
    assert_eq!(items.len(), 1);

    let endpoint = &items[0];
    assert_eq!(endpoint.name, "index");
    assert_eq!(endpoint.kind, "rest");
    assert_eq!(endpoint.path, Some("/".to_string()));
    assert_eq!(endpoint.language, Some("rust".to_string()));
    assert!(endpoint.framework.is_none());
    assert_eq!(endpoint.line, 5);
}

#[test]
fn extracts_actix_post_endpoint() {
    let analyzer = RustAnalyzer;
    let source = r#"
use actix_web::{Responder, HttpResponse};

#[actix_web::post("/items")]
async fn create_item() -> impl Responder {
    HttpResponse::Ok().finish()
}
"#;

    let items =
        analyzer.find_endpoints(Path::new("src/handlers.rs"), source, &any_endpoint_query());
    assert_eq!(items.len(), 1);

    let endpoint = &items[0];
    assert_eq!(endpoint.name, "create_item");
    assert_eq!(endpoint.kind, "rest");
    assert_eq!(endpoint.path, Some("/items".to_string()));
}

#[test]
fn extracts_actix_endpoint_with_short_attribute() {
    let analyzer = RustAnalyzer;
    let source = r#"
use actix_web::Responder;

#[get("/items")]
async fn list_items() -> impl Responder {
    "Items"
}
"#;

    let items = analyzer.find_endpoints(Path::new("src/routes.rs"), source, &any_endpoint_query());
    assert_eq!(items.len(), 1);

    let endpoint = &items[0];
    assert_eq!(endpoint.name, "list_items");
    assert_eq!(endpoint.kind, "rest");
    assert_eq!(endpoint.path, Some("/items".to_string()));
}

#[test]
fn extracts_multiple_actix_endpoints() {
    let analyzer = RustAnalyzer;
    let source = r#"
use actix_web::Responder;

#[get("/users")]
async fn list_users() -> impl Responder {
    "users"
}

#[post("/users")]
async fn create_user() -> impl Responder {
    "created"
}

#[get("/users/{id}")]
async fn get_user() -> impl Responder {
    "user"
}

#[delete("/users/{id}")]
async fn delete_user() -> impl Responder {
    "deleted"
}
"#;

    let items = analyzer.find_endpoints(Path::new("src/users.rs"), source, &any_endpoint_query());
    assert_eq!(items.len(), 4);

    let names: Vec<&str> = items.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"list_users"));
    assert!(names.contains(&"create_user"));
    assert!(names.contains(&"get_user"));
    assert!(names.contains(&"delete_user"));

    // Verify kinds
    for item in &items {
        assert_eq!(item.kind, "rest");
    }
}

#[test]
fn filters_by_rest_kind() {
    let analyzer = RustAnalyzer;
    let source = r#"
use actix_web::Responder;

#[get("/items")]
async fn get_items() -> impl Responder {
    "items"
}

#[post("/items")]
async fn create_item() -> impl Responder {
    "created"
}
"#;

    let items =
        analyzer.find_endpoints(Path::new("src/handlers.rs"), source, &rest_endpoint_query());
    assert_eq!(items.len(), 2);

    for item in &items {
        assert_eq!(item.kind, "rest");
    }
}

#[test]
fn extracts_async_graphql_object_methods() {
    let analyzer = RustAnalyzer;
    let source = r#"
use async_graphql::Object;

pub struct QueryRoot;

#[async_graphql::Object]
impl QueryRoot {
    async fn get_user(&self, id: String) -> String {
        format!("User {}", id)
    }

    async fn list_users(&self) -> Vec<String> {
        vec!["user1".to_string()]
    }
}
"#;

    let items = analyzer.find_endpoints(
        Path::new("src/graphql/query.rs"),
        source,
        &any_endpoint_query(),
    );
    assert_eq!(items.len(), 2);

    let names: Vec<&str> = items.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"get_user"));
    assert!(names.contains(&"list_users"));

    for item in &items {
        assert_eq!(item.kind, "graphql");
        assert!(item.path.is_none());
    }
}

#[test]
fn extracts_async_graphql_subscription_methods() {
    let analyzer = RustAnalyzer;
    let source = r#"
use async_graphql::Subscription;

pub struct SubscriptionRoot;

#[async_graphql::Subscription]
impl SubscriptionRoot {
    async fn on_user_created(&self) -> String {
        "user created".to_string()
    }
}
"#;

    let items = analyzer.find_endpoints(
        Path::new("src/graphql/subscription.rs"),
        source,
        &graphql_endpoint_query(),
    );
    assert_eq!(items.len(), 1);

    let endpoint = &items[0];
    assert_eq!(endpoint.name, "on_user_created");
    assert_eq!(endpoint.kind, "graphql");
    assert!(endpoint.path.is_none());
}

#[test]
fn separates_rest_and_graphql_endpoints() {
    let analyzer = RustAnalyzer;
    let source = r#"
use actix_web::Responder;
use async_graphql::Object;

#[get("/api/users")]
async fn get_users() -> impl Responder {
    "users"
}

pub struct QueryRoot;

#[async_graphql::Object]
impl QueryRoot {
    async fn search(&self) -> String {
        "search result".to_string()
    }
}
"#;

    let all_items = analyzer.find_endpoints(Path::new("src/lib.rs"), source, &any_endpoint_query());
    assert_eq!(all_items.len(), 2);

    let rest_items =
        analyzer.find_endpoints(Path::new("src/lib.rs"), source, &rest_endpoint_query());
    assert_eq!(rest_items.len(), 1);
    assert_eq!(rest_items[0].name, "get_users");

    let graphql_items =
        analyzer.find_endpoints(Path::new("src/lib.rs"), source, &graphql_endpoint_query());
    assert_eq!(graphql_items.len(), 1);
    assert_eq!(graphql_items[0].name, "search");
}

#[test]
fn respects_limit_parameter() {
    let analyzer = RustAnalyzer;
    let source = r#"
use actix_web::Responder;

#[get("/one")]
async fn one() -> impl Responder { "one" }

#[get("/two")]
async fn two() -> impl Responder { "two" }

#[get("/three")]
async fn three() -> impl Responder { "three" }
"#;

    let limited_query = FindEndpointsQuery {
        kind: "any".to_string(),
        public_language_filter: None,
        public_framework_filter: None,
        limit: 2,
    };

    let items = analyzer.find_endpoints(Path::new("src/lib.rs"), source, &limited_query);
    assert_eq!(items.len(), 2);
}

#[test]
fn supports_framework_returns_true_for_none() {
    let analyzer = RustAnalyzer;
    assert!(analyzer.supports_framework(None));
    assert!(!analyzer.supports_framework(Some("actix")));
    assert!(!analyzer.supports_framework(Some("axum")));
}
