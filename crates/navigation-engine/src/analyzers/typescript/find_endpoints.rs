use std::path::Path;

use tree_sitter::{Node, Parser};

use super::super::types::{
    infer_public_language, normalize_public_endpoint_kind, EndpointDefinition, FindEndpointsQuery,
};
use super::common::{derive_route_path_from_file, node_text, parser_language_for_path};

pub(super) fn find_endpoints(
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

fn collect_endpoints(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
    route_path: Option<&str>,
    is_route_file: bool,
    endpoints: &mut Vec<EndpointDefinition>,
) {
    if !is_route_file {
        return;
    }

    if node.kind() == "export_statement" {
        if let Some(endpoint) = extract_endpoint(node, source, public_language, route_path) {
            endpoints.push(endpoint);
        }
        return;
    }

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
    let (name, raw_kind) = match node.kind() {
        "function_declaration" | "generator_function_declaration" => {
            let name_node = node.child_by_field_name("name")?;
            let name = node_text(name_node, source)?;
            if name != "loader" && name != "action" {
                return None;
            }
            let kind = name.clone();
            (name, kind)
        }
        "variable_declarator" => {
            let name_node = node.child_by_field_name("name")?;
            let name = node_text(name_node, source)?;
            if name != "loader" && name != "action" {
                return None;
            }
            let value = node.child_by_field_name("value")?;
            if !matches!(value.kind(), "arrow_function" | "function_expression") {
                return None;
            }
            let kind = name.clone();
            (name, kind)
        }
        "export_statement" => {
            let declaration = node.child_by_field_name("declaration")?;
            match declaration.kind() {
                "function_declaration"
                | "generator_function_declaration"
                | "variable_declarator" => {
                    return extract_endpoint(declaration, source, public_language, route_path);
                }
                "lexical_declaration" => {
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
