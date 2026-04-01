use std::collections::BTreeSet;
use std::path::Path;

use tree_sitter::{Node, Parser};

use super::language_analyzer::LanguageAnalyzer;
use super::types::{
    infer_public_language, normalize_public_endpoint_kind, AnalyzerLanguage, CalleeDefinition,
    EndpointDefinition, FindCalleesQuery, FindEndpointsQuery, FindSymbolQuery, SymbolDefinition,
};

pub struct PythonAnalyzer;

impl LanguageAnalyzer for PythonAnalyzer {
    fn language(&self) -> AnalyzerLanguage {
        AnalyzerLanguage::Python
    }

    fn supported_extensions(&self) -> &'static [&'static str] {
        &[".py"]
    }

    fn find_symbols(
        &self,
        path: &Path,
        source: &str,
        _query: &FindSymbolQuery,
    ) -> Vec<SymbolDefinition> {
        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .is_err()
        {
            return Vec::new();
        }

        let Some(tree) = parser.parse(source, None) else {
            return Vec::new();
        };

        let mut symbols = Vec::new();
        let public_language = infer_public_language(path);
        collect_symbols(
            tree.root_node(),
            source.as_bytes(),
            public_language.as_deref(),
            &mut symbols,
        );

        dedupe_symbols(symbols)
    }

    fn find_endpoints(
        &self,
        path: &Path,
        source: &str,
        query: &FindEndpointsQuery,
    ) -> Vec<EndpointDefinition> {
        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .is_err()
        {
            return Vec::new();
        }

        let Some(tree) = parser.parse(source, None) else {
            return Vec::new();
        };

        let public_language = infer_public_language(path);
        let mut endpoints = Vec::new();
        collect_endpoints(
            tree.root_node(),
            source.as_bytes(),
            public_language.as_deref(),
            &mut endpoints,
        );

        // Filter by kind
        let filtered: Vec<EndpointDefinition> = endpoints
            .into_iter()
            .filter(|e| {
                if query.kind == "any" {
                    true
                } else {
                    e.kind == query.kind
                }
            })
            .take(query.limit)
            .collect();

        filtered
    }

    fn find_callees(
        &self,
        path: &Path,
        source: &str,
        query: &FindCalleesQuery,
    ) -> Vec<CalleeDefinition> {
        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .is_err()
        {
            return Vec::new();
        }

        let Some(tree) = parser.parse(source, None) else {
            return Vec::new();
        };

        let public_language = infer_public_language(path);
        let mut callees = Vec::new();
        let ctx = PythonCalleeContext {
            target_symbol: &query.target_symbol,
            current_file: path,
            public_language: public_language.as_deref(),
        };

        collect_python_callees(
            tree.root_node(),
            source.as_bytes(),
            None,
            &ctx,
            &mut callees,
        );
        callees
    }

    fn supports_framework(&self, framework: Option<&str>) -> bool {
        // Python has no specific framework filter (supports all)
        framework.is_none()
    }
}

fn collect_symbols(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
    symbols: &mut Vec<SymbolDefinition>,
) {
    if let Some(symbol) = extract_symbol(node, source, public_language) {
        symbols.push(symbol);
    }

    for index in 0..node.named_child_count() {
        if let Some(child) = node.named_child(index) {
            collect_symbols(child, source, public_language, symbols);
        }
    }
}

fn extract_symbol(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
) -> Option<SymbolDefinition> {
    let effective_node = unwrap_decorated_definition(node)?;
    let raw_kind = match effective_node.kind() {
        "class_definition" => "class",
        "function_definition" | "async_function_definition" => {
            classify_function_kind(effective_node)?
        }
        _ => return None,
    };

    let name_node = effective_node.child_by_field_name("name")?;

    Some(SymbolDefinition {
        symbol: node_text(name_node, source)?,
        kind: raw_kind.to_string(),
        path: String::new(),
        line: (effective_node.start_position().row + 1) as u32,
        line_end: (effective_node.end_position().row + 1) as u32,
        language: public_language.map(str::to_string),
    })
}

fn unwrap_decorated_definition(node: Node) -> Option<Node> {
    if node.kind() != "decorated_definition" {
        return Some(node);
    }

    for index in 0..node.named_child_count() {
        let child = node.named_child(index)?;
        if child.kind() != "decorator" {
            return Some(child);
        }
    }

    None
}

fn classify_function_kind(node: Node) -> Option<&'static str> {
    let mut current = node.parent();
    let mut inside_class = false;

    while let Some(parent) = current {
        match parent.kind() {
            "function_definition" | "async_function_definition" => return None,
            "class_definition" => inside_class = true,
            _ => {}
        }
        current = parent.parent();
    }

    Some(if inside_class { "method" } else { "function" })
}

fn dedupe_symbols(symbols: Vec<SymbolDefinition>) -> Vec<SymbolDefinition> {
    let mut seen = BTreeSet::new();
    let mut deduped = Vec::new();

    for symbol in symbols {
        let key = (
            symbol.symbol.clone(),
            symbol.kind.clone(),
            symbol.line,
            symbol.language.clone(),
        );

        if seen.insert(key) {
            deduped.push(symbol);
        }
    }

    deduped
}

fn node_text(node: Node, source: &[u8]) -> Option<String> {
    node.utf8_text(source)
        .ok()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

/// Collects FastAPI, Flask, and Django endpoints from Python source.
fn collect_endpoints(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
    endpoints: &mut Vec<EndpointDefinition>,
) {
    // Check for decorated_definition (FastAPI/Flask decorators)
    if node.kind() == "decorated_definition" {
        // Extract all endpoints from all decorators on this function
        let found = extract_decorator_endpoints(node, source, public_language);
        endpoints.extend(found);
    }

    // Check for call nodes (Django path()/url() calls can appear anywhere)
    if node.kind() == "call" {
        if let Some(endpoint) = extract_django_call_endpoint(&node, source, public_language) {
            endpoints.push(endpoint);
        }
    }

    // Recursively traverse the tree
    for index in 0..node.named_child_count() {
        if let Some(child) = node.named_child(index) {
            collect_endpoints(child, source, public_language, endpoints);
        }
    }
}

/// Extracts all endpoints from a decorated_definition node (FastAPI/Flask).
/// Pattern: @app.get("/path") or @app.route("/path")
/// Returns endpoints for ALL decorators on the function.
fn extract_decorator_endpoints(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
) -> Vec<EndpointDefinition> {
    let mut results = Vec::new();

    // Find the function_definition inside decorated_definition
    let Some(function_def) = find_function_definition(node) else {
        return results;
    };

    // Get function name
    let Some(name_node) = function_def.child_by_field_name("name") else {
        return results;
    };
    let Some(name) = node_text(name_node, source) else {
        return results;
    };

    // Find ALL decorators and extract HTTP method + path from each
    for index in 0..node.named_child_count() {
        let Some(child) = node.named_child(index) else {
            continue;
        };
        if child.kind() == "decorator" {
            if let Some((http_method, path)) = extract_decorator_info(child, source) {
                results.push(EndpointDefinition {
                    name: name.clone(),
                    kind: normalize_public_endpoint_kind(&http_method),
                    path: Some(path),
                    file: String::new(),
                    line: (node.start_position().row + 1) as u32,
                    language: public_language.map(str::to_string),
                    framework: None,
                });
            }
        }
    }

    results
}

/// Finds the function_definition inside a decorated_definition.
fn find_function_definition(node: Node) -> Option<Node> {
    for index in 0..node.named_child_count() {
        let child = node.named_child(index)?;
        if matches!(
            child.kind(),
            "function_definition" | "async_function_definition"
        ) {
            return Some(child);
        }
    }
    None
}

/// Extracts HTTP method and path from a decorator node.
/// Handles both @app.get("/path") and @app.route("/path") patterns.
fn extract_decorator_info(node: Node, source: &[u8]) -> Option<(String, String)> {
    // decorator node's named child is the call/attribute directly (not a field named "expression")
    for index in 0..node.named_child_count() {
        let child = node.named_child(index)?;
        match child.kind() {
            "call" => {
                let function = child.child_by_field_name("function")?;
                return extract_route_call(&function, &child, source);
            }
            "attribute" => {
                // @app.route without call - not a valid route decorator for our purposes
                continue;
            }
            _ => continue,
        }
    }
    None
}

/// Extracts route info from a call expression like app.get("/path").
fn extract_route_call(
    function: &Node,
    call_node: &Node,
    source: &[u8],
) -> Option<(String, String)> {
    // function should be an attribute like app.get
    if function.kind() != "attribute" {
        return None;
    }

    let attr_node = function.child_by_field_name("attribute")?;
    let method_name = node_text(attr_node, source)?;

    // Check if this is a routing decorator
    let http_methods = [
        "get",
        "post",
        "put",
        "delete",
        "patch",
        "route",
        "api_route",
    ];
    if !http_methods.contains(&method_name.as_str()) {
        return None;
    }

    // Extract the path from arguments
    let path = extract_first_string_argument(call_node, source)?;

    Some((method_name, path))
}

/// Extracts the first string argument from a call expression.
fn extract_first_string_argument(call_node: &Node, source: &[u8]) -> Option<String> {
    let args = call_node.child_by_field_name("arguments")?;

    // Look for string arguments (Python tree-sitter uses "string" for string literals)
    for index in 0..args.named_child_count() {
        let arg = args.named_child(index)?;
        if arg.kind() == "string" {
            return extract_string_value(&arg, source);
        }
    }

    None
}

/// Extracts the string value from a string node (handles string_fragment).
fn extract_string_value(node: &Node, source: &[u8]) -> Option<String> {
    if node.kind() != "string" {
        return None;
    }

    // Python string node contains string_fragment child
    for index in 0..node.named_child_count() {
        let child = node.named_child(index)?;
        if child.kind() == "string_fragment" {
            return node_text(child, source);
        }
        if child.kind() == "interpolation" {
            // f-string, skip for simplicity
            return None;
        }
    }

    // Fallback: try to get text directly from the string node and strip quotes
    let text = node_text(*node, source)?;
    let trimmed = text.trim_start_matches('"').trim_end_matches('"');
    let trimmed = trimmed.trim_start_matches('\'').trim_end_matches('\'');
    if trimmed.is_empty() || trimmed.len() == text.len() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
/// Extracts endpoint from a Django path() or url() call.
fn extract_django_call_endpoint(
    call_node: &Node,
    source: &[u8],
    public_language: Option<&str>,
) -> Option<EndpointDefinition> {
    let function = call_node.child_by_field_name("function")?;

    // Check if this is path() or url() call
    let func_name = node_text(function, source)?;
    if !matches!(func_name.as_str(), "path" | "url") {
        return None;
    }

    let args = call_node.child_by_field_name("arguments")?;

    // First argument is the path pattern
    let path = extract_nth_string_argument(&args, source, 0)?;

    // Second argument is the view - extract its name
    let view_name = extract_view_name(&args, source)?;

    Some(EndpointDefinition {
        name: view_name,
        kind: normalize_public_endpoint_kind("route"),
        path: Some(path),
        file: String::new(),
        line: (call_node.start_position().row + 1) as u32,
        language: public_language.map(str::to_string),
        framework: None,
    })
}

/// Extracts the nth string argument from an argument list.
fn extract_nth_string_argument(args: &Node, source: &[u8], n: usize) -> Option<String> {
    let mut string_count = 0;

    for index in 0..args.named_child_count() {
        let arg = args.named_child(index)?;
        if arg.kind() == "string" {
            if string_count == n {
                return extract_string_value(&arg, source);
            }
            string_count += 1;
        }
    }

    None
}

/// Extracts the view name from the second argument of a Django path() call.
/// Handles both attribute (views.article_list) and identifier (article_list) patterns.
fn extract_view_name(args: &Node, source: &[u8]) -> Option<String> {
    let mut arg_index = 0;

    for index in 0..args.named_child_count() {
        let arg = args.named_child(index)?;
        // Skip string arguments (the path pattern)
        if arg.kind() == "string" {
            arg_index += 1;
            continue;
        }

        // The view is typically the second non-keyword argument
        if arg_index == 1 {
            return match arg.kind() {
                // views.article_list
                "attribute" => {
                    let attr = arg.child_by_field_name("attribute")?;
                    node_text(attr, source)
                }
                // article_list (identifier)
                "identifier" => node_text(arg, source),
                _ => None,
            };
        }
    }

    None
}

struct PythonCalleeContext<'a> {
    target_symbol: &'a str,
    current_file: &'a Path,
    public_language: Option<&'a str>,
}

#[derive(Clone)]
struct PythonFunctionContext {
    name: String,
}

fn collect_python_callees(
    node: Node,
    source: &[u8],
    current_function: Option<PythonFunctionContext>,
    ctx: &PythonCalleeContext,
    callees: &mut Vec<CalleeDefinition>,
) {
    // Check if this node is a function definition we're looking for
    let is_target_function = match node.kind() {
        "function_definition" | "async_function_definition" => node
            .child_by_field_name("name")
            .and_then(|n| python_node_text(n, source))
            .map(|name| name == ctx.target_symbol)
            .unwrap_or(false),
        _ => false,
    };

    let next_function = if is_target_function || current_function.is_some() {
        let name = node
            .child_by_field_name("name")
            .and_then(|n| python_node_text(n, source))
            .unwrap_or_default();
        Some(PythonFunctionContext { name })
    } else {
        current_function.clone()
    };

    // If we're inside the target function, look for call expressions
    if is_target_function || current_function.is_some() {
        if node.kind() == "call" {
            if let Some(callee) = extract_python_callee(node, source, ctx) {
                callees.push(callee);
            }
        }
    }

    // Recurse into children
    for index in 0..node.named_child_count() {
        if let Some(child) = node.named_child(index) {
            collect_python_callees(child, source, next_function.clone(), ctx, callees);
        }
    }
}

fn extract_python_callee(
    node: Node,
    source: &[u8],
    ctx: &PythonCalleeContext,
) -> Option<CalleeDefinition> {
    if node.kind() != "call" {
        return None;
    }

    let function = node.child_by_field_name("function")?;

    let (callee_name, receiver_type) = match function.kind() {
        "identifier" => {
            let name = python_node_text(function, source)?;
            (name, None)
        }
        "attribute" => {
            let object = function
                .child_by_field_name("object")
                .and_then(|n| python_node_text(n, source));
            let attr = function
                .child_by_field_name("attribute")
                .and_then(|n| python_node_text(n, source));
            let name =
                attr.unwrap_or_else(|| python_node_text(function, source).unwrap_or_default());
            (name, object)
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
        snippet: python_node_text(node, source),
    })
}

fn python_node_text(node: Node, source: &[u8]) -> Option<String> {
    node.utf8_text(source)
        .ok()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}
