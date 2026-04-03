use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnalyzerLanguage {
    Auto,
    Go,
    Java,
    Python,
    Rust,
    Typescript,
}

#[derive(Debug, Clone)]
pub struct FindSymbolQuery {
    pub symbol: String,
    pub kind: String,
    pub match_mode: String,
    pub public_language_filter: Option<String>,
    pub limit: usize,
}

#[derive(Debug, Clone)]
pub struct FindEndpointsQuery {
    pub kind: String,
    pub public_language_filter: Option<String>,
    pub public_framework_filter: Option<String>,
    pub limit: usize,
}

#[derive(Debug, Clone)]
pub struct FindCallersQuery {
    pub target_path: PathBuf,
    pub target_symbol: String,
}

#[derive(Debug, Clone)]
pub struct FindCalleesQuery {
    pub target_symbol: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalleeDefinition {
    pub path: String,
    pub line: u32,
    pub end_line: u32,
    pub column: Option<u32>,
    pub callee: String,
    pub callee_symbol: Option<String>,
    pub receiver_type: Option<String>,
    pub relation: String,
    pub language: Option<String>,
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolDefinition {
    pub symbol: String,
    pub kind: String,
    pub path: String,
    pub line: u32,
    pub line_end: u32,
    pub language: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EndpointDefinition {
    pub name: String,
    pub kind: String,
    pub path: Option<String>,
    pub file: String,
    pub line: u32,
    pub language: Option<String>,
    pub framework: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallerTarget {
    pub path: String,
    pub symbol: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallerDefinition {
    pub path: String,
    pub line: u32,
    pub column: Option<u32>,
    pub caller: String,
    pub caller_symbol: Option<String>,
    pub relation: String,
    pub language: Option<String>,
    pub snippet: Option<String>,
    pub receiver_type: Option<String>,
    pub calls: CallerTarget,
    pub probable_entry_point_reasons: Vec<String>,
}

pub fn file_extension(path: &Path) -> Option<String> {
    path.extension()
        .map(|value| format!(".{}", value.to_string_lossy().to_lowercase()))
}

pub fn infer_public_language(path: &Path) -> Option<String> {
    match file_extension(path).as_deref() {
        Some(".ts") | Some(".tsx") => Some("typescript".to_string()),
        Some(".js") | Some(".jsx") => Some("javascript".to_string()),
        Some(".go") => Some("go".to_string()),
        Some(".java") => Some("java".to_string()),
        Some(".py") => Some("python".to_string()),
        Some(".rs") => Some("rust".to_string()),
        _ => None,
    }
}

pub fn normalize_public_symbol_kind(raw_kind: &str) -> String {
    match raw_kind {
        "class" | "class_declaration" => "class",
        "interface" | "interface_declaration" => "interface",
        "function" | "function_declaration" => "function",
        "method" | "method_declaration" | "method_signature" => "method",
        "type" | "type_alias" | "type_alias_declaration" | "record" => "type",
        "enum" | "enum_declaration" => "enum",
        "constructor" | "constructor_declaration" => "constructor",
        "annotation" | "annotation_type" => "annotation",
        _ => "any",
    }
    .to_string()
}

/// Normalizes internal endpoint kinds to public endpoint kinds.
/// Maps source-level identifiers to unified kinds:
/// - "loader", "action" → "route" (React Router 7)
/// - "@GetMapping", "@PostMapping", etc. → "rest" (Spring)
/// - "@QueryMapping", "@MutationMapping" → "graphql" (Spring GraphQL)
/// - "get", "post", "put", "delete", "patch" → "rest" (FastAPI, Flask, Actix, Axum)
pub fn normalize_public_endpoint_kind(raw_kind: &str) -> String {
    match raw_kind {
        // React Router 7
        "loader" | "action" => "route",
        // Spring REST
        "@GetMapping" | "@PostMapping" | "@PutMapping" | "@DeleteMapping" | "@PatchMapping"
        | "@RequestMapping" | "GetMapping" | "PostMapping" | "PutMapping" | "DeleteMapping"
        | "PatchMapping" | "RequestMapping" => "rest",
        // Spring GraphQL
        "@QueryMapping"
        | "@MutationMapping"
        | "@SubscriptionMapping"
        | "QueryMapping"
        | "MutationMapping"
        | "SubscriptionMapping" => "graphql",
        // FastAPI, Flask, Axum route methods
        "get" | "post" | "put" | "delete" | "patch" | "route" => "rest",
        // async-graphql
        "Object" | "Subscription" => "graphql",
        _ => "any",
    }
    .to_string()
}
