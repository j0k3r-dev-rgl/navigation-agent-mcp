use std::path::Path;

use tree_sitter::{Node, Parser};

use super::language_analyzer::LanguageAnalyzer;
use super::types::{
    infer_public_language, normalize_public_endpoint_kind, AnalyzerLanguage, CalleeDefinition,
    EndpointDefinition, FindCalleesQuery, FindEndpointsQuery, FindSymbolQuery, SymbolDefinition,
};

pub struct RustAnalyzer;

impl LanguageAnalyzer for RustAnalyzer {
    fn language(&self) -> AnalyzerLanguage {
        AnalyzerLanguage::Rust
    }

    fn supported_extensions(&self) -> &'static [&'static str] {
        &[".rs"]
    }

    fn find_symbols(
        &self,
        path: &Path,
        source: &str,
        _query: &FindSymbolQuery,
    ) -> Vec<SymbolDefinition> {
        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .is_err()
        {
            return Vec::new();
        }

        let Some(tree) = parser.parse(source, None) else {
            return Vec::new();
        };

        let public_language = infer_public_language(path);
        let mut symbols = Vec::new();
        collect_source_file_symbols(
            tree.root_node(),
            source.as_bytes(),
            public_language.as_deref(),
            &mut symbols,
        );
        symbols
    }

    fn find_endpoints(
        &self,
        path: &Path,
        source: &str,
        query: &FindEndpointsQuery,
    ) -> Vec<EndpointDefinition> {
        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .is_err()
        {
            return Vec::new();
        }

        let Some(tree) = parser.parse(source, None) else {
            return Vec::new();
        };

        let public_language = infer_public_language(path);
        let mut endpoints = Vec::new();
        collect_source_file_endpoints(
            tree.root_node(),
            source.as_bytes(),
            public_language.as_deref(),
            &mut endpoints,
        );

        endpoints
            .into_iter()
            .filter(|endpoint| query.kind == "any" || endpoint.kind == query.kind)
            .take(query.limit)
            .collect()
    }

    fn find_callees(
        &self,
        path: &Path,
        source: &str,
        query: &FindCalleesQuery,
    ) -> Vec<CalleeDefinition> {
        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .is_err()
        {
            return Vec::new();
        }

        let Some(tree) = parser.parse(source, None) else {
            return Vec::new();
        };

        let public_language = infer_public_language(path);
        let mut callees = Vec::new();
        let ctx = RustCalleeContext {
            target_symbol: &query.target_symbol,
            current_file: path,
            public_language: public_language.as_deref(),
        };

        collect_rust_callees(
            tree.root_node(),
            source.as_bytes(),
            None,
            &ctx,
            &mut callees,
        );
        callees
    }

    fn supports_framework(&self, framework: Option<&str>) -> bool {
        framework.is_none()
    }
}

fn collect_source_file_endpoints(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
    endpoints: &mut Vec<EndpointDefinition>,
) {
    match node.kind() {
        "function_item" => {
            if let Some(endpoint) = extract_rest_endpoint(node, source, public_language) {
                endpoints.push(endpoint);
            }
        }
        "impl_item" => {
            endpoints.extend(extract_graphql_endpoints(node, source, public_language));
        }
        _ => {}
    }

    for index in 0..node.named_child_count() {
        let Some(child) = node.named_child(index) else {
            continue;
        };

        collect_source_file_endpoints(child, source, public_language, endpoints);
    }
}

fn collect_source_file_symbols(
    root: Node,
    source: &[u8],
    public_language: Option<&str>,
    symbols: &mut Vec<SymbolDefinition>,
) {
    for index in 0..root.named_child_count() {
        let Some(child) = root.named_child(index) else {
            continue;
        };

        match child.kind() {
            "struct_item" => push_named_symbol(child, source, public_language, "type", symbols),
            "enum_item" => push_named_symbol(child, source, public_language, "enum", symbols),
            "trait_item" => push_named_symbol(child, source, public_language, "interface", symbols),
            "type_item" => push_named_symbol(child, source, public_language, "type", symbols),
            "function_item" => {
                push_named_symbol(child, source, public_language, "function", symbols)
            }
            "impl_item" => collect_impl_methods(child, source, public_language, symbols),
            _ => {}
        }
    }
}

fn collect_impl_methods(
    impl_item: Node,
    source: &[u8],
    public_language: Option<&str>,
    symbols: &mut Vec<SymbolDefinition>,
) {
    let Some(body) = impl_body(impl_item) else {
        return;
    };

    for index in 0..body.named_child_count() {
        let Some(child) = body.named_child(index) else {
            continue;
        };

        if child.kind() == "function_item" {
            push_named_symbol(child, source, public_language, "method", symbols);
        }
    }
}

fn impl_body(node: Node) -> Option<Node> {
    node.child_by_field_name("body").or_else(|| {
        (0..node.named_child_count())
            .filter_map(|index| node.named_child(index))
            .find(|child| child.kind() == "declaration_list")
    })
}

fn push_named_symbol(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
    kind: &str,
    symbols: &mut Vec<SymbolDefinition>,
) {
    if let Some(symbol) = build_named_symbol(node, source, public_language, kind) {
        symbols.push(symbol);
    }
}

fn build_named_symbol(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
    kind: &str,
) -> Option<SymbolDefinition> {
    let name_node = node.child_by_field_name("name")?;

    Some(SymbolDefinition {
        symbol: node_text(name_node, source)?,
        kind: kind.to_string(),
        path: String::new(),
        line: (node.start_position().row + 1) as u32,
        line_end: (node.end_position().row + 1) as u32,
        language: public_language.map(str::to_string),
    })
}

fn node_text(node: Node, source: &[u8]) -> Option<String> {
    node.utf8_text(source)
        .ok()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn extract_rest_endpoint(
    function_item: Node,
    source: &[u8],
    public_language: Option<&str>,
) -> Option<EndpointDefinition> {
    let name_node = function_item.child_by_field_name("name")?;
    let name = node_text(name_node, source)?;

    for attribute in leading_attribute_items(function_item) {
        let Some((http_method, path)) = extract_rest_attribute(attribute, source) else {
            continue;
        };

        return Some(EndpointDefinition {
            name,
            kind: normalize_public_endpoint_kind(&http_method),
            path: Some(path),
            file: String::new(),
            line: (name_node.start_position().row + 1) as u32,
            language: public_language.map(str::to_string),
            framework: None,
        });
    }

    None
}

fn extract_graphql_endpoints(
    impl_item: Node,
    source: &[u8],
    public_language: Option<&str>,
) -> Vec<EndpointDefinition> {
    let Some(kind) = graphql_impl_kind(impl_item, source) else {
        return Vec::new();
    };

    let Some(body) = impl_body(impl_item) else {
        return Vec::new();
    };

    let mut endpoints = Vec::new();
    for index in 0..body.named_child_count() {
        let Some(child) = body.named_child(index) else {
            continue;
        };
        if child.kind() != "function_item" {
            continue;
        }

        let Some(name_node) = child.child_by_field_name("name") else {
            continue;
        };
        let Some(name) = node_text(name_node, source) else {
            continue;
        };

        endpoints.push(EndpointDefinition {
            name,
            kind: normalize_public_endpoint_kind(kind),
            path: None,
            file: String::new(),
            line: (name_node.start_position().row + 1) as u32,
            language: public_language.map(str::to_string),
            framework: None,
        });
    }

    endpoints
}

fn extract_rest_attribute(attribute_item: Node, source: &[u8]) -> Option<(String, String)> {
    let text = node_text(attribute_item, source)?;
    let inner = text.strip_prefix("#[")?.strip_suffix(']')?.trim();
    let (macro_name, args) = inner.split_once('(')?;
    let method = macro_name.rsplit("::").next()?.trim();

    if !matches!(method, "get" | "post" | "put" | "delete" | "patch") {
        return None;
    }

    Some((method.to_string(), extract_first_quoted_string(args)?))
}

fn leading_attribute_items(node: Node) -> Vec<Node> {
    let mut attributes = Vec::new();
    let mut current = node.prev_named_sibling();

    while let Some(sibling) = current {
        if sibling.kind() != "attribute_item" {
            break;
        }

        attributes.push(sibling);
        current = sibling.prev_named_sibling();
    }

    attributes.reverse();
    attributes
}

fn graphql_macro_name(attribute: Node, source: &[u8]) -> Option<String> {
    let text = node_text(attribute, source)?;
    let inner = text.strip_prefix("#[")?.strip_suffix(']')?.trim();
    let macro_name = inner.split_once('(').map(|(name, _)| name).unwrap_or(inner);
    Some(macro_name.rsplit("::").next()?.trim().to_string())
}

fn graphql_impl_kind(impl_item: Node, source: &[u8]) -> Option<&'static str> {
    for attribute in leading_attribute_items(impl_item) {
        let Some(macro_name) = graphql_macro_name(attribute, source) else {
            continue;
        };

        match macro_name.as_str() {
            "Object" => return Some("Object"),
            "Subscription" => return Some("Subscription"),
            _ => continue,
        }
    }

    None
}

fn extract_first_quoted_string(text: &str) -> Option<String> {
    let start = text.find('"')?;
    let rest = &text[start + 1..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::FindSymbolQuery;

    fn any_query() -> FindSymbolQuery {
        FindSymbolQuery {
            symbol: "load".to_string(),
            kind: "any".to_string(),
            match_mode: "fuzzy".to_string(),
            public_language_filter: None,
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
}

struct RustCalleeContext<'a> {
    target_symbol: &'a str,
    current_file: &'a Path,
    public_language: Option<&'a str>,
}

#[derive(Clone)]
struct RustFunctionContext {
    name: String,
}

fn collect_rust_callees(
    node: Node,
    source: &[u8],
    current_function: Option<RustFunctionContext>,
    ctx: &RustCalleeContext,
    callees: &mut Vec<CalleeDefinition>,
) {
    // Check if this node is a function_item we're looking for
    let is_target_function = node.kind() == "function_item"
        && node
            .child_by_field_name("name")
            .and_then(|n| rust_node_text(n, source))
            .map(|name| name == ctx.target_symbol)
            .unwrap_or(false);

    let next_function = if is_target_function || current_function.is_some() {
        let name = node
            .child_by_field_name("name")
            .and_then(|n| rust_node_text(n, source))
            .unwrap_or_default();
        Some(RustFunctionContext { name })
    } else {
        current_function.clone()
    };

    // If we're inside the target function, look for call expressions
    if is_target_function || current_function.is_some() {
        if node.kind() == "call_expression" {
            if let Some(callee) = extract_rust_callee(node, source, ctx) {
                callees.push(callee);
            }
        }
        // Also check for method calls
        if node.kind() == "method_call_expr" {
            if let Some(callee) = extract_rust_callee(node, source, ctx) {
                callees.push(callee);
            }
        }
    }

    // Recurse into children
    for index in 0..node.named_child_count() {
        if let Some(child) = node.named_child(index) {
            collect_rust_callees(child, source, next_function.clone(), ctx, callees);
        }
    }
}

fn extract_rust_callee(
    node: Node,
    source: &[u8],
    ctx: &RustCalleeContext,
) -> Option<CalleeDefinition> {
    let (callee_name, receiver_type) = match node.kind() {
        "call_expression" => {
            let func = node.child_by_field_name("function")?;
            rust_node_text(func, source)
                .map(|name| (name, None))
                .unwrap_or_default()
        }
        "method_call_expr" => {
            let receiver = node
                .child_by_field_name("receiver")
                .and_then(|n| rust_node_text(n, source));
            let method = node
                .child_by_field_name("method")
                .and_then(|n| rust_node_text(n, source))?;
            (method, receiver)
        }
        _ => return None,
    };

    let end_line = (node.end_position().row + 1) as u32;

    Some(CalleeDefinition {
        path: ctx.current_file.to_string_lossy().replace('\\', "/"),
        line: (node.start_position().row + 1) as u32,
        end_line,
        column: Some((node.start_position().column + 1) as u32),
        callee: callee_name,
        callee_symbol: None,
        receiver_type,
        relation: "calls".to_string(),
        language: ctx.public_language.map(String::from),
        snippet: rust_node_text(node, source),
    })
}

fn rust_node_text(node: Node, source: &[u8]) -> Option<String> {
    node.utf8_text(source)
        .ok()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}
