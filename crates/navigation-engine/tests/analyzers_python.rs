use std::path::Path;

use navigation_engine::analyzers::language_analyzer::LanguageAnalyzer;
use navigation_engine::analyzers::python::PythonAnalyzer;
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

#[test]
fn extracts_supported_python_definitions() {
    let analyzer = PythonAnalyzer;
    let source = r#"
class Worker:
    @classmethod
    def build(cls):
        return cls()

    async def fetch(self):
        return 1

def load_data():
    return 1

async def fetch_users():
    return []

@decorator
def serialize():
    return "ok"

@decorator
class DecoratedService:
    pass

class DecoratedMethods:
    @staticmethod
    @audit
    def save():
        return True
"#;

    let items = analyzer.find_symbols(Path::new("profiles/service.py"), source, &any_query());
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

    assert!(kinds.contains(&("Worker", "class", Some("python"))));
    assert!(kinds.contains(&("build", "method", Some("python"))));
    assert!(kinds.contains(&("fetch", "method", Some("python"))));
    assert!(kinds.contains(&("load_data", "function", Some("python"))));
    assert!(kinds.contains(&("fetch_users", "function", Some("python"))));
    assert!(kinds.contains(&("serialize", "function", Some("python"))));
    assert!(kinds.contains(&("DecoratedService", "class", Some("python"))));
    assert!(kinds.contains(&("save", "method", Some("python"))));

    assert_eq!(
        items
            .iter()
            .filter(|item| item.symbol == "serialize")
            .count(),
        1
    );
    assert_eq!(
        items
            .iter()
            .filter(|item| item.symbol == "DecoratedService")
            .count(),
        1
    );
    assert_eq!(items.iter().filter(|item| item.symbol == "save").count(), 1);
}

#[test]
fn excludes_unsupported_python_constructs() {
    let analyzer = PythonAnalyzer;
    let source = r#"
value = lambda: 1
from users import load_alias

def outer():
    def inner():
        return 1
    return inner
"#;

    let items = analyzer.find_symbols(Path::new("profiles/unsupported.py"), source, &any_query());
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].symbol, "outer");
    assert_eq!(items[0].kind, "function");
    assert!(items.iter().all(|item| item.symbol != "inner"));
    assert!(items.iter().all(|item| item.symbol != "load_alias"));
}

#[test]
fn extracts_fastapi_get_endpoint() {
    let analyzer = PythonAnalyzer;
    let source = r#"
from fastapi import FastAPI

app = FastAPI()

@app.get("/items")
def read_items():
    return {"items": []}
"#;

    let endpoints =
        analyzer.find_endpoints(Path::new("api/main.py"), source, &any_endpoint_query());

    assert_eq!(endpoints.len(), 1);
    assert_eq!(endpoints[0].name, "read_items");
    assert_eq!(endpoints[0].kind, "rest");
    assert_eq!(endpoints[0].path, Some("/items".to_string()));
    assert_eq!(endpoints[0].language, Some("python".to_string()));
    assert!(endpoints[0].framework.is_none());
}

#[test]
fn extracts_fastapi_post_endpoint() {
    let analyzer = PythonAnalyzer;
    let source = r#"
from fastapi import FastAPI

app = FastAPI()

@app.post("/items")
def create_item(item: dict):
    return item
"#;

    let endpoints =
        analyzer.find_endpoints(Path::new("api/main.py"), source, &any_endpoint_query());

    assert_eq!(endpoints.len(), 1);
    assert_eq!(endpoints[0].name, "create_item");
    assert_eq!(endpoints[0].kind, "rest");
    assert_eq!(endpoints[0].path, Some("/items".to_string()));
}

#[test]
fn extracts_flask_route_endpoint() {
    let analyzer = PythonAnalyzer;
    let source = r#"
from flask import Flask

app = Flask(__name__)

@app.route("/hello")
def hello():
    return "Hello, World!"
"#;

    let endpoints = analyzer.find_endpoints(Path::new("api/app.py"), source, &any_endpoint_query());

    assert_eq!(endpoints.len(), 1);
    assert_eq!(endpoints[0].name, "hello");
    assert_eq!(endpoints[0].kind, "rest");
    assert_eq!(endpoints[0].path, Some("/hello".to_string()));
}

#[test]
fn extracts_django_path_endpoint() {
    let analyzer = PythonAnalyzer;
    let source = r#"
from django.urls import path
from . import views

urlpatterns = [
    path("articles", views.article_list),
    path("articles/<int:id>", views.article_detail),
]
"#;

    let endpoints =
        analyzer.find_endpoints(Path::new("api/urls.py"), source, &any_endpoint_query());

    assert_eq!(endpoints.len(), 2);
    assert_eq!(endpoints[0].name, "article_list");
    assert_eq!(endpoints[0].kind, "rest");
    assert_eq!(endpoints[0].path, Some("articles".to_string()));

    assert_eq!(endpoints[1].name, "article_detail");
    assert_eq!(endpoints[1].kind, "rest");
    assert_eq!(endpoints[1].path, Some("articles/<int:id>".to_string()));
}

#[test]
fn extracts_multiple_http_methods() {
    let analyzer = PythonAnalyzer;
    let source = r#"
from fastapi import FastAPI

app = FastAPI()

@app.get("/users")
def list_users():
    return []

@app.post("/users")
def create_user():
    return {}

@app.put("/users/{user_id}")
def update_user(user_id: int):
    return {}

@app.delete("/users/{user_id}")
def delete_user(user_id: int):
    return {}
"#;

    let endpoints =
        analyzer.find_endpoints(Path::new("api/users.py"), source, &any_endpoint_query());

    assert_eq!(endpoints.len(), 4);

    let names: Vec<&str> = endpoints.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"list_users"));
    assert!(names.contains(&"create_user"));
    assert!(names.contains(&"update_user"));
    assert!(names.contains(&"delete_user"));

    let paths: Vec<Option<&str>> = endpoints.iter().map(|e| e.path.as_deref()).collect();
    assert!(paths.contains(&Some("/users")));
    assert!(paths.contains(&Some("/users/{user_id}")));
}

#[test]
fn multiple_decorators_single_function() {
    let analyzer = PythonAnalyzer;
    let source = r#"
from fastapi import FastAPI

app = FastAPI()

@app.get("/items")
@app.post("/items")
def handle_items():
    return {}
"#;

    let endpoints =
        analyzer.find_endpoints(Path::new("api/main.py"), source, &any_endpoint_query());

    // Should extract one endpoint per decorator
    assert_eq!(endpoints.len(), 2);
    assert_eq!(endpoints[0].name, "handle_items");
    assert_eq!(endpoints[0].path, Some("/items".to_string()));
    assert_eq!(endpoints[1].name, "handle_items");
    assert_eq!(endpoints[1].path, Some("/items".to_string()));
}

#[test]
fn filters_by_rest_kind() {
    let analyzer = PythonAnalyzer;
    let source = r#"
from fastapi import FastAPI

app = FastAPI()

@app.get("/items")
def read_items():
    return []
"#;

    let endpoints =
        analyzer.find_endpoints(Path::new("api/main.py"), source, &rest_endpoint_query());

    assert_eq!(endpoints.len(), 1);
    assert_eq!(endpoints[0].name, "read_items");
    assert_eq!(endpoints[0].kind, "rest");
}

#[test]
fn respects_query_limit() {
    let analyzer = PythonAnalyzer;
    let source = r#"
from fastapi import FastAPI

app = FastAPI()

@app.get("/items")
def read_items():
    return []

@app.post("/items")
def create_item():
    return {}

@app.put("/items/{id}")
def update_item(id: int):
    return {}
"#;

    let query = FindEndpointsQuery {
        kind: "any".to_string(),
        public_language_filter: None,
        public_framework_filter: None,
        limit: 2,
    };

    let endpoints = analyzer.find_endpoints(Path::new("api/main.py"), source, &query);
    assert_eq!(endpoints.len(), 2);
}

#[test]
fn no_false_positives_on_non_web_decorators() {
    let analyzer = PythonAnalyzer;
    let source = r#"
@dataclass
class User:
    name: str

@cache
def get_data():
    return None

@log_execution
def process():
    pass

@app.custom_method("/internal")
def internal_handler():
    pass
"#;

    let endpoints =
        analyzer.find_endpoints(Path::new("services/user.py"), source, &any_endpoint_query());

    // Should not extract any endpoints from non-web decorators
    assert_eq!(endpoints.len(), 0);
}

#[test]
fn supports_framework_returns_false_for_any_framework() {
    let analyzer = PythonAnalyzer;
    // Python analyzer should only support None (no framework filter)
    assert!(analyzer.supports_framework(None));
    assert!(!analyzer.supports_framework(Some("fastapi")));
    assert!(!analyzer.supports_framework(Some("flask")));
    assert!(!analyzer.supports_framework(Some("django")));
}
