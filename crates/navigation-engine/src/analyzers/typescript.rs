use std::path::Path;

use tree_sitter::{Node, Parser};

use super::language_analyzer::LanguageAnalyzer;
use super::types::{
    infer_public_language, normalize_public_endpoint_kind, normalize_public_symbol_kind,
    AnalyzerLanguage, EndpointDefinition, FindEndpointsQuery, FindSymbolQuery, SymbolDefinition,
};

pub struct TypeScriptAnalyzer;

impl LanguageAnalyzer for TypeScriptAnalyzer {
    fn language(&self) -> AnalyzerLanguage {
        AnalyzerLanguage::Typescript
    }

    fn supported_extensions(&self) -> &'static [&'static str] {
        &[".ts", ".tsx", ".js", ".jsx"]
    }

    fn find_symbols(
        &self,
        path: &Path,
        source: &str,
        _query: &FindSymbolQuery,
    ) -> Vec<SymbolDefinition> {
        let Some(language) = parser_language_for_path(path) else {
            return Vec::new();
        };

        let mut parser = Parser::new();
        if parser.set_language(&language.into()).is_err() {
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
        _query: &FindEndpointsQuery,
    ) -> Vec<EndpointDefinition> {
        let Some(language) = parser_language_for_path(path) else {
            return Vec::new();
        };

        let mut parser = Parser::new();
        if parser.set_language(&language.into()).is_err() {
            return Vec::new();
        }

        let Some(tree) = parser.parse(source, None) else {
            return Vec::new();
        };

        let public_language = infer_public_language(path);
        let route_path = derive_route_path_from_file(path);
        let is_route_file = route_path.is_some();

        let mut endpoints = Vec::new();
        collect_endpoints(
            tree.root_node(),
            source.as_bytes(),
            public_language.as_deref(),
            route_path.as_deref(),
            is_route_file,
            &mut endpoints,
        );
        endpoints
    }

    fn supports_framework(&self, framework: Option<&str>) -> bool {
        match framework {
            None => true,
            Some("react-router") => true,
            Some(_) => false,
        }
    }
}

fn parser_language_for_path(path: &Path) -> Option<tree_sitter_language::LanguageFn> {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
    {
        Some(extension) if extension == "ts" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT),
        Some(extension) if extension == "tsx" => Some(tree_sitter_typescript::LANGUAGE_TSX),
        Some(extension) if extension == "js" || extension == "jsx" => {
            Some(tree_sitter_javascript::LANGUAGE)
        }
        _ => None,
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
        "function_declaration" | "generator_function_declaration" => {
            (node.child_by_field_name("name")?, "function_declaration")
        }
        "class_declaration" | "abstract_class_declaration" => {
            (node.child_by_field_name("name")?, "class_declaration")
        }
        "interface_declaration" => (node.child_by_field_name("name")?, "interface_declaration"),
        "enum_declaration" => (node.child_by_field_name("name")?, "enum_declaration"),
        "type_alias_declaration" => (node.child_by_field_name("name")?, "type_alias_declaration"),
        "method_definition" | "method_signature" | "abstract_method_signature" => {
            let name_node = node.child_by_field_name("name")?;
            let symbol = node_text(name_node, source)?;
            let raw_kind = if symbol == "constructor" {
                "constructor"
            } else {
                "method_declaration"
            };

            return Some(SymbolDefinition {
                symbol,
                kind: normalize_public_symbol_kind(raw_kind),
                path: String::new(),
                line: (node.start_position().row + 1) as u32,
                language: public_language.map(str::to_string),
            });
        }
        "variable_declarator" => {
            let value = node.child_by_field_name("value")?;
            if !matches!(value.kind(), "arrow_function" | "function_expression") {
                return None;
            }
            (node.child_by_field_name("name")?, "function_declaration")
        }
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

/// Derives the React Router 7 route path from a file path.
/// Convention mapping:
/// - `app/routes/dashboard.tsx` → `/dashboard`
/// - `app/routes/users.$id.tsx` → `/users/:id`
/// - `app/routes/_index.tsx` → `/`
/// - `app/routes/admin.users.tsx` → `/admin/users`
/// - `app/routes/admin._index.tsx` → `/admin`
fn derive_route_path_from_file(path: &Path) -> Option<String> {
    let path_str = path.to_string_lossy();

    // Check if this is a routes directory file
    let routes_idx = path_str.find("/routes/")?;
    let route_file = &path_str[routes_idx + 8..]; // Skip "/routes/"

    // Remove extension
    let route_name = route_file.rsplit_once('.')?.0;

    // Handle _index route
    if route_name == "_index" || route_name.ends_with("/_index") {
        let parent = route_name.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
        if parent.is_empty() {
            return Some("/".to_string());
        }
        return Some(format!("/{}", parent.replace('.', "/")));
    }

    // Convert route segments
    let segments: Vec<&str> = route_name
        .split('/')
        .last()
        .unwrap_or(route_name)
        .split('.')
        .collect();

    let path_segments: Vec<String> = segments
        .iter()
        .filter(|s| !s.starts_with('_')) // Skip layout markers
        .map(|s| {
            if s.starts_with('$') {
                // Dynamic segment: $id -> :id
                format!(":{}", &s[1..])
            } else {
                s.to_string()
            }
        })
        .collect();

    if path_segments.is_empty() {
        return Some("/".to_string());
    }

    Some(format!("/{}", path_segments.join("/")))
}

fn collect_endpoints(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
    route_path: Option<&str>,
    is_route_file: bool,
    endpoints: &mut Vec<EndpointDefinition>,
) {
    // Only extract endpoints from route files (files in app/routes/)
    if !is_route_file {
        return;
    }

    // Handle export statements - this is where exported loaders/actions live
    if node.kind() == "export_statement" {
        if let Some(endpoint) = extract_endpoint(node, source, public_language, route_path) {
            endpoints.push(endpoint);
        }
        return;
    }

    // Recursively traverse the tree
    for index in 0..node.named_child_count() {
        if let Some(child) = node.named_child(index) {
            collect_endpoints(
                child,
                source,
                public_language,
                route_path,
                is_route_file,
                endpoints,
            );
        }
    }
}

fn extract_endpoint(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
    route_path: Option<&str>,
) -> Option<EndpointDefinition> {
    // Look for exported loader or action functions
    let (name, raw_kind) = match node.kind() {
        "function_declaration" | "generator_function_declaration" => {
            let name_node = node.child_by_field_name("name")?;
            let name = node_text(name_node, source)?;

            // Only interested in loader and action for RR7 routes
            if name != "loader" && name != "action" {
                return None;
            }
            let kind = name.clone();
            (name, kind)
        }
        "variable_declarator" => {
            let name_node = node.child_by_field_name("name")?;
            let name = node_text(name_node, source)?;

            // Only interested in loader and action for RR7 routes
            if name != "loader" && name != "action" {
                return None;
            }

            // Check if the value is a function
            let value = node.child_by_field_name("value")?;
            if !matches!(value.kind(), "arrow_function" | "function_expression") {
                return None;
            }
            let kind = name.clone();
            (name, kind)
        }
        "export_statement" => {
            // Handle export function loader() {} or export const loader = () => {}
            // The declaration could be a function_declaration or lexical_declaration
            let declaration = node.child_by_field_name("declaration")?;
            match declaration.kind() {
                "function_declaration"
                | "generator_function_declaration"
                | "variable_declarator" => {
                    return extract_endpoint(declaration, source, public_language, route_path);
                }
                "lexical_declaration" => {
                    // export const loader = ... -> lexical_declaration contains variable_declarator
                    for i in 0..declaration.named_child_count() {
                        if let Some(child) = declaration.named_child(i) {
                            if child.kind() == "variable_declarator" {
                                return extract_endpoint(
                                    child,
                                    source,
                                    public_language,
                                    route_path,
                                );
                            }
                        }
                    }
                    return None;
                }
                _ => return None,
            }
        }
        _ => return None,
    };

    Some(EndpointDefinition {
        name,
        kind: normalize_public_endpoint_kind(&raw_kind),
        path: route_path.map(str::to_string),
        file: String::new(),
        line: (node.start_position().row + 1) as u32,
        language: public_language.map(str::to_string),
        framework: Some("react-router".to_string()),
    })
}
