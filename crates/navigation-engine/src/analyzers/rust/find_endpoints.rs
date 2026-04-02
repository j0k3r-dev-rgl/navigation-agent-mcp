use std::path::Path;

use tree_sitter::{Node, Parser};

use super::super::types::{
    infer_public_language, normalize_public_endpoint_kind, EndpointDefinition, FindEndpointsQuery,
};
use super::common::{impl_body, node_text};

pub(super) fn find_endpoints(
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
