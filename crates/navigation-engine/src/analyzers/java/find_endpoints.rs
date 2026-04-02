use std::path::Path;

use tree_sitter::{Node, Parser};

use super::super::types::{
    infer_public_language, normalize_public_endpoint_kind, EndpointDefinition, FindEndpointsQuery,
};
use super::common::{
    extract_annotation_name, extract_marker_annotation_name, find_modifiers_child, node_text,
};

pub(super) fn find_endpoints(
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

    endpoints
        .into_iter()
        .filter(|e| {
            if query.kind == "any" {
                true
            } else {
                e.kind == query.kind
            }
        })
        .take(query.limit)
        .collect()
}

fn collect_endpoints(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
    endpoints: &mut Vec<EndpointDefinition>,
) {
    if node.kind() != "class_declaration" {
        for index in 0..node.named_child_count() {
            if let Some(child) = node.named_child(index) {
                collect_endpoints(child, source, public_language, endpoints);
            }
        }
        return;
    }

    let modifiers = find_modifiers_child(&node);
    let (is_rest_controller, is_graphql_controller) = match &modifiers {
        Some(m) => check_controller_type(m, source),
        None => (false, false),
    };

    if !is_rest_controller && !is_graphql_controller {
        return;
    }

    let base_path = modifiers
        .as_ref()
        .and_then(|m| extract_request_mapping_path(m, source));

    let class_body = match node.child_by_field_name("body") {
        Some(b) => b,
        None => return,
    };

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
                "marker_annotation" => (extract_marker_annotation_name(&child, source), None),
                "annotation" => (
                    extract_annotation_name(&child, source),
                    extract_annotation_path(&child, source),
                ),
                _ => continue,
            };

            if let Some(name) = annotation_name {
                if rest_annotations.contains(&name.as_str()) {
                    let method_name = method_node
                        .child_by_field_name("name")
                        .and_then(|n| node_text(n, source))
                        .unwrap_or_default();

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
                    let method_name = method_node
                        .child_by_field_name("name")
                        .and_then(|n| node_text(n, source))
                        .unwrap_or_default();

                    endpoints.push(EndpointDefinition {
                        name: method_name,
                        kind: normalize_public_endpoint_kind(&name),
                        path: None,
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

fn combine_paths(base: Option<&str>, method: Option<&str>) -> Option<String> {
    match (base, method) {
        (Some(base), Some(method)) => {
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

fn check_controller_type(modifiers: &Node, source: &[u8]) -> (bool, bool) {
    let mut is_rest = false;
    let mut is_graphql = false;

    for index in 0..modifiers.named_child_count() {
        if let Some(child) = modifiers.named_child(index) {
            let annotation_name = match child.kind() {
                "marker_annotation" => extract_marker_annotation_name(&child, source),
                "annotation" => extract_annotation_name(&child, source),
                _ => None,
            };

            match annotation_name.as_deref() {
                Some("RestController") => is_rest = true,
                Some("Controller") => is_graphql = true,
                _ => {}
            }
        }
    }

    (is_rest, is_graphql)
}

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

fn extract_annotation_path(node: &Node, source: &[u8]) -> Option<String> {
    let args = node.child_by_field_name("arguments")?;

    for index in 0..args.named_child_count() {
        if let Some(arg) = args.named_child(index) {
            if arg.kind() == "string_literal" {
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
