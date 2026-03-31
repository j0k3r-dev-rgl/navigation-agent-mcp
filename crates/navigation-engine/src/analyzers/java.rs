use std::path::Path;

use tree_sitter::{Node, Parser};

use super::language_analyzer::LanguageAnalyzer;
use super::types::{
    infer_public_language, normalize_public_endpoint_kind, normalize_public_symbol_kind,
    AnalyzerLanguage, CallerDefinition, CallerTarget, EndpointDefinition, FindCallersQuery,
    FindEndpointsQuery, FindSymbolQuery, SymbolDefinition,
};

pub struct JavaAnalyzer;

impl LanguageAnalyzer for JavaAnalyzer {
    fn language(&self) -> AnalyzerLanguage {
        AnalyzerLanguage::Java
    }

    fn supported_extensions(&self) -> &'static [&'static str] {
        &[".java"]
    }

    fn find_symbols(
        &self,
        path: &Path,
        source: &str,
        _query: &FindSymbolQuery,
    ) -> Vec<SymbolDefinition> {
        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_java::LANGUAGE.into())
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
            .set_language(&tree_sitter_java::LANGUAGE.into())
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

    fn find_callers(
        &self,
        _workspace_root: &Path,
        path: &Path,
        source: &str,
        query: &FindCallersQuery,
    ) -> Vec<CallerDefinition> {
        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_java::LANGUAGE.into())
            .is_err()
        {
            return Vec::new();
        }

        let Some(tree) = parser.parse(source, None) else {
            return Vec::new();
        };

        let public_language = infer_public_language(path);
        let mut callers = Vec::new();
        collect_java_callers(
            tree.root_node(),
            source.as_bytes(),
            public_language.as_deref(),
            query,
            None,
            None,
            &mut callers,
        );
        callers
    }

    fn supports_framework(&self, framework: Option<&str>) -> bool {
        match framework {
            None => true,
            Some("spring") => true,
            Some(_) => false,
        }
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
    let (name_node, raw_kind) = match node.kind() {
        "class_declaration" => (node.child_by_field_name("name")?, "class_declaration"),
        "interface_declaration" => (node.child_by_field_name("name")?, "interface_declaration"),
        "enum_declaration" => (node.child_by_field_name("name")?, "enum_declaration"),
        "annotation_type_declaration" => (node.child_by_field_name("name")?, "annotation_type"),
        "record_declaration" => (node.child_by_field_name("name")?, "record"),
        "method_declaration" => (node.child_by_field_name("name")?, "method_declaration"),
        "constructor_declaration" => (node.child_by_field_name("name")?, "constructor_declaration"),
        _ => return None,
    };

    Some(SymbolDefinition {
        symbol: node_text(name_node, source)?,
        kind: normalize_public_symbol_kind(raw_kind),
        path: String::new(),
        line: (node.start_position().row + 1) as u32,
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

/// Collects Spring REST and GraphQL endpoints from Java class declarations.
fn collect_endpoints(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
    endpoints: &mut Vec<EndpointDefinition>,
) {
    // Only process class_declaration nodes
    if node.kind() != "class_declaration" {
        for index in 0..node.named_child_count() {
            if let Some(child) = node.named_child(index) {
                collect_endpoints(child, source, public_language, endpoints);
            }
        }
        return;
    }

    // Get class modifiers - it's a child node, not a field
    let modifiers = find_modifiers_child(&node);

    // Determine controller type (REST or GraphQL)
    let (is_rest_controller, is_graphql_controller) = match &modifiers {
        Some(m) => check_controller_type(m, source),
        None => (false, false),
    };

    if !is_rest_controller && !is_graphql_controller {
        return;
    }

    // Extract class-level @RequestMapping path (base path)
    let base_path = modifiers
        .as_ref()
        .and_then(|m| extract_request_mapping_path(m, source));

    // Find class_body and traverse method declarations
    let class_body = match node.child_by_field_name("body") {
        Some(b) => b,
        None => return,
    };

    // Process all method declarations in the class body
    for index in 0..class_body.named_child_count() {
        if let Some(child) = class_body.named_child(index) {
            if child.kind() == "method_declaration" {
                if is_rest_controller {
                    extract_rest_endpoints(
                        &child,
                        source,
                        public_language,
                        base_path.as_deref(),
                        endpoints,
                    );
                }
                if is_graphql_controller {
                    extract_graphql_endpoints(&child, source, public_language, endpoints);
                }
            }
        }
    }
}

/// Finds the modifiers child node in a class_declaration or method_declaration.
fn find_modifiers_child<'a>(node: &Node<'a>) -> Option<Node<'a>> {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "modifiers" {
                return Some(child);
            }
        }
    }
    None
}

/// Checks if a class is a REST controller (@RestController) or GraphQL controller (@Controller).
fn check_controller_type(modifiers: &Node, source: &[u8]) -> (bool, bool) {
    let mut is_rest_controller = false;
    let mut is_graphql_controller = false;

    for index in 0..modifiers.named_child_count() {
        if let Some(child) = modifiers.named_child(index) {
            let annotation_name = match child.kind() {
                "marker_annotation" => extract_marker_annotation_name(&child, source),
                "annotation" => extract_annotation_name(&child, source),
                _ => None,
            };

            if let Some(name) = annotation_name {
                match name.as_str() {
                    "RestController" => is_rest_controller = true,
                    "Controller" => is_graphql_controller = true,
                    _ => {}
                }
            }
        }
    }

    (is_rest_controller, is_graphql_controller)
}

/// Extracts the path from a class-level @RequestMapping annotation.
fn extract_request_mapping_path(modifiers: &Node, source: &[u8]) -> Option<String> {
    for index in 0..modifiers.named_child_count() {
        if let Some(child) = modifiers.named_child(index) {
            if child.kind() == "annotation" {
                let name = extract_annotation_name(&child, source);
                if name.as_deref() == Some("RequestMapping") {
                    return extract_annotation_path(&child, source);
                }
            }
        }
    }
    None
}

/// Extracts the annotation name from a marker_annotation node.
fn extract_marker_annotation_name(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| node_text(n, source))
}

/// Extracts the annotation name from an annotation node.
fn extract_annotation_name(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| node_text(n, source))
}

/// Extracts the path string from an annotation's argument list.
fn extract_annotation_path(node: &Node, source: &[u8]) -> Option<String> {
    let args = node.child_by_field_name("arguments")?;

    // Look for string_literal in the arguments
    for index in 0..args.named_child_count() {
        if let Some(arg) = args.named_child(index) {
            if arg.kind() == "string_literal" {
                // string_literal contains string_fragment
                for i in 0..arg.named_child_count() {
                    if let Some(fragment) = arg.named_child(i) {
                        if fragment.kind() == "string_fragment" {
                            return node_text(fragment, source);
                        }
                    }
                }
            }
        }
    }
    None
}

/// Extracts REST endpoints from a method declaration.
fn extract_rest_endpoints(
    method_node: &Node,
    source: &[u8],
    public_language: Option<&str>,
    base_path: Option<&str>,
    endpoints: &mut Vec<EndpointDefinition>,
) {
    let modifiers = match find_modifiers_child(method_node) {
        Some(m) => m,
        None => return,
    };

    // Check for REST mapping annotations
    let rest_annotations = [
        "GetMapping",
        "PostMapping",
        "PutMapping",
        "DeleteMapping",
        "PatchMapping",
        "RequestMapping",
    ];

    for index in 0..modifiers.named_child_count() {
        if let Some(child) = modifiers.named_child(index) {
            let (annotation_name, path) = match child.kind() {
                "marker_annotation" => {
                    let name = extract_marker_annotation_name(&child, source);
                    (name, None)
                }
                "annotation" => {
                    let name = extract_annotation_name(&child, source);
                    let path = extract_annotation_path(&child, source);
                    (name, path)
                }
                _ => continue,
            };

            if let Some(name) = annotation_name {
                if rest_annotations.contains(&name.as_str()) {
                    // Get method name
                    let method_name = method_node
                        .child_by_field_name("name")
                        .and_then(|n| node_text(n, source))
                        .unwrap_or_default();

                    // Combine base path with method path
                    let full_path = combine_paths(base_path, path.as_deref());

                    endpoints.push(EndpointDefinition {
                        name: method_name,
                        kind: normalize_public_endpoint_kind(&name),
                        path: full_path,
                        file: String::new(),
                        line: (method_node.start_position().row + 1) as u32,
                        language: public_language.map(str::to_string),
                        framework: Some("spring".to_string()),
                    });
                }
            }
        }
    }
}

/// Extracts GraphQL endpoints from a method declaration.
fn extract_graphql_endpoints(
    method_node: &Node,
    source: &[u8],
    public_language: Option<&str>,
    endpoints: &mut Vec<EndpointDefinition>,
) {
    let modifiers = match find_modifiers_child(method_node) {
        Some(m) => m,
        None => return,
    };

    // GraphQL mapping annotations
    let graphql_annotations = ["QueryMapping", "MutationMapping", "SubscriptionMapping"];

    for index in 0..modifiers.named_child_count() {
        if let Some(child) = modifiers.named_child(index) {
            let annotation_name = match child.kind() {
                "marker_annotation" => extract_marker_annotation_name(&child, source),
                "annotation" => extract_annotation_name(&child, source),
                _ => continue,
            };

            if let Some(name) = annotation_name {
                if graphql_annotations.contains(&name.as_str()) {
                    // Get method name (field name in GraphQL schema)
                    let method_name = method_node
                        .child_by_field_name("name")
                        .and_then(|n| node_text(n, source))
                        .unwrap_or_default();

                    // GraphQL field name is the method name (no path combination)
                    let graphql_field = method_name.clone();

                    endpoints.push(EndpointDefinition {
                        name: graphql_field,
                        kind: normalize_public_endpoint_kind(&name),
                        path: None, // GraphQL has no path, field name is the identifier
                        file: String::new(),
                        line: (method_node.start_position().row + 1) as u32,
                        language: public_language.map(str::to_string),
                        framework: Some("spring".to_string()),
                    });
                }
            }
        }
    }
}

/// Combines base path and method path.
/// Examples:
/// - ("/titulares", "/{id}") → "/titulares/{id}"
/// - ("/titulares", None) → "/titulares"
/// - (None, "/items") → "/items"
fn combine_paths(base: Option<&str>, method: Option<&str>) -> Option<String> {
    match (base, method) {
        (Some(base), Some(method)) => {
            // Ensure proper path concatenation
            let base = base.trim_start_matches('/');
            let method = method.trim_start_matches('/');
            if method.is_empty() {
                Some(format!("/{}", base))
            } else {
                Some(format!("/{}/{}", base, method))
            }
        }
        (Some(base), None) => Some(format!("/{}", base.trim_start_matches('/'))),
        (None, Some(method)) => Some(format!("/{}", method.trim_start_matches('/'))),
        (None, None) => None,
    }
}

fn collect_java_callers(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
    query: &FindCallersQuery,
    current_class: Option<String>,
    current_method: Option<(String, Vec<String>)>,
    callers: &mut Vec<CallerDefinition>,
) {
    let next_class = if node.kind() == "class_declaration" {
        node.child_by_field_name("name")
            .and_then(|item| node_text(item, source))
            .or(current_class.clone())
    } else {
        current_class.clone()
    };

    let next_method = if matches!(
        node.kind(),
        "method_declaration" | "constructor_declaration"
    ) {
        let name = node
            .child_by_field_name("name")
            .and_then(|item| node_text(item, source));
        name.map(|method_name| {
            let caller_display = next_class
                .as_ref()
                .map(|class_name| format!("{}#{}", class_name, method_name))
                .unwrap_or_else(|| method_name.clone());
            let reasons = extract_probable_entry_point_reasons(node, source);
            (caller_display, reasons)
        })
    } else {
        current_method.clone()
    };

    if node.kind() == "method_invocation" {
        if let Some(caller) = extract_java_call(node, source, public_language, query, &next_method)
        {
            callers.push(caller);
        }
    }

    for index in 0..node.named_child_count() {
        if let Some(child) = node.named_child(index) {
            collect_java_callers(
                child,
                source,
                public_language,
                query,
                next_class.clone(),
                next_method.clone(),
                callers,
            );
        }
    }
}

fn extract_java_call(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
    query: &FindCallersQuery,
    current_method: &Option<(String, Vec<String>)>,
) -> Option<CallerDefinition> {
    let name = node
        .child_by_field_name("name")
        .and_then(|item| node_text(item, source))?;
    if name != query.target_symbol {
        return None;
    }

    let (caller_display, reasons) = current_method.as_ref()?.clone();
    let receiver_type = node
        .child_by_field_name("object")
        .and_then(|item| node_text(item, source));

    Some(CallerDefinition {
        path: String::new(),
        line: (node.start_position().row + 1) as u32,
        column: Some((node.start_position().column + 1) as u32),
        caller: caller_display.clone(),
        caller_symbol: Some(caller_display),
        relation: "calls".to_string(),
        language: public_language.map(str::to_string),
        snippet: node_text(node, source),
        receiver_type: receiver_type.clone(),
        calls: CallerTarget {
            path: query.target_path.to_string_lossy().replace('\\', "/"),
            symbol: query.target_symbol.clone(),
        },
        probable_entry_point_reasons: reasons,
    })
}

fn extract_probable_entry_point_reasons(node: Node, source: &[u8]) -> Vec<String> {
    let Some(modifiers) = find_modifiers_child(&node) else {
        return Vec::new();
    };

    let mut reasons = Vec::new();
    for index in 0..modifiers.named_child_count() {
        let Some(child) = modifiers.named_child(index) else {
            continue;
        };
        let annotation_name = match child.kind() {
            "marker_annotation" => extract_marker_annotation_name(&child, source),
            "annotation" => extract_annotation_name(&child, source),
            _ => None,
        };

        match annotation_name.as_deref() {
            Some("GetMapping")
            | Some("PostMapping")
            | Some("PutMapping")
            | Some("DeleteMapping")
            | Some("PatchMapping")
            | Some("RequestMapping") => reasons.push("public controller method".to_string()),
            Some("QueryMapping") | Some("MutationMapping") | Some("SubscriptionMapping") => {
                reasons.push("public graphql method".to_string())
            }
            _ => {}
        }
    }
    reasons
}
