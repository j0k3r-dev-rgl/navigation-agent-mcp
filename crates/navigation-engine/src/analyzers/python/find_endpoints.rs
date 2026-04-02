use std::path::Path;

use tree_sitter::{Node, Parser};

use super::super::types::{
    infer_public_language, normalize_public_endpoint_kind, EndpointDefinition, FindEndpointsQuery,
};
use super::common::node_text;

pub(super) fn find_endpoints(
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
    if node.kind() == "decorated_definition" {
        let found = extract_decorator_endpoints(node, source, public_language);
        endpoints.extend(found);
    }

    if node.kind() == "call" {
        if let Some(endpoint) = extract_django_call_endpoint(&node, source, public_language) {
            endpoints.push(endpoint);
        }
    }

    for index in 0..node.named_child_count() {
        if let Some(child) = node.named_child(index) {
            collect_endpoints(child, source, public_language, endpoints);
        }
    }
}

fn extract_decorator_endpoints(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
) -> Vec<EndpointDefinition> {
    let mut results = Vec::new();

    let Some(function_def) = find_function_definition(node) else {
        return results;
    };

    let Some(name_node) = function_def.child_by_field_name("name") else {
        return results;
    };
    let Some(name) = node_text(name_node, source) else {
        return results;
    };

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

fn extract_decorator_info(node: Node, source: &[u8]) -> Option<(String, String)> {
    for index in 0..node.named_child_count() {
        let child = node.named_child(index)?;
        match child.kind() {
            "call" => {
                let function = child.child_by_field_name("function")?;
                return extract_route_call(&function, &child, source);
            }
            "attribute" => continue,
            _ => continue,
        }
    }
    None
}

fn extract_route_call(
    function: &Node,
    call_node: &Node,
    source: &[u8],
) -> Option<(String, String)> {
    if function.kind() != "attribute" {
        return None;
    }

    let attr_node = function.child_by_field_name("attribute")?;
    let method_name = node_text(attr_node, source)?;

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

    let path = extract_first_string_argument(call_node, source)?;
    Some((method_name, path))
}

fn extract_first_string_argument(call_node: &Node, source: &[u8]) -> Option<String> {
    let args = call_node.child_by_field_name("arguments")?;

    for index in 0..args.named_child_count() {
        let arg = args.named_child(index)?;
        if arg.kind() == "string" {
            return extract_string_value(&arg, source);
        }
    }

    None
}

fn extract_string_value(node: &Node, source: &[u8]) -> Option<String> {
    if node.kind() != "string" {
        return None;
    }

    for index in 0..node.named_child_count() {
        let child = node.named_child(index)?;
        if child.kind() == "string_fragment" {
            return node_text(child, source);
        }
        if child.kind() == "interpolation" {
            return None;
        }
    }

    let text = node_text(*node, source)?;
    let trimmed = text.trim_start_matches('"').trim_end_matches('"');
    let trimmed = trimmed.trim_start_matches('\'').trim_end_matches('\'');
    if trimmed.is_empty() || trimmed.len() == text.len() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn extract_django_call_endpoint(
    call_node: &Node,
    source: &[u8],
    public_language: Option<&str>,
) -> Option<EndpointDefinition> {
    let function = call_node.child_by_field_name("function")?;

    let func_name = node_text(function, source)?;
    if !matches!(func_name.as_str(), "path" | "url") {
        return None;
    }

    let args = call_node.child_by_field_name("arguments")?;
    let path = extract_nth_string_argument(&args, source, 0)?;
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

fn extract_view_name(args: &Node, source: &[u8]) -> Option<String> {
    let mut arg_index = 0;

    for index in 0..args.named_child_count() {
        let arg = args.named_child(index)?;
        if arg.kind() == "string" {
            arg_index += 1;
            continue;
        }

        if arg_index == 1 {
            return match arg.kind() {
                "attribute" => {
                    let attr = arg.child_by_field_name("attribute")?;
                    node_text(attr, source)
                }
                "identifier" => node_text(arg, source),
                _ => None,
            };
        }
    }

    None
}
