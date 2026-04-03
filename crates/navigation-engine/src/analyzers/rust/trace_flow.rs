use std::collections::HashMap;
use std::path::Path;

use tree_sitter::{Node, Parser};

use super::super::types::{infer_public_language, CalleeDefinition, FindCalleesQuery};
use super::common::node_text;

pub(super) struct RustCalleeContext<'a> {
    pub(super) target_symbol: &'a str,
    pub(super) current_file: &'a Path,
    pub(super) public_language: Option<&'a str>,
}

#[derive(Clone)]
struct RustFunctionContext {
    local_bindings: HashMap<String, RustBindingMeta>,
}

#[derive(Clone)]
struct RustBindingMeta {
    owner_type: String,
}

impl RustBindingMeta {
    fn owner_type(&self) -> &str {
        self.owner_type.as_str()
    }
}

pub(super) fn find_callees(
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

fn collect_rust_callees(
    node: Node,
    source: &[u8],
    current_function: Option<RustFunctionContext>,
    ctx: &RustCalleeContext,
    callees: &mut Vec<CalleeDefinition>,
) {
    let is_target_function = node.kind() == "function_item"
        && node
            .child_by_field_name("name")
            .and_then(|n| rust_node_text(n, source))
            .map(|name| {
                let qualified = qualify_rust_function_name(node, &name, source);
                name == ctx.target_symbol || qualified.as_deref() == Some(ctx.target_symbol)
            })
            .unwrap_or(false);

    let next_function = if is_target_function {
        let owner_name = enclosing_impl_owner(node, source);
        let local_bindings = collect_rust_local_bindings(node, source, owner_name.as_deref());
        Some(RustFunctionContext { local_bindings })
    } else if current_function.is_some() {
        current_function.clone()
    } else {
        None
    };

    if is_target_function || current_function.is_some() {
        if node.kind() == "call_expression" {
            if let Some(callee) = extract_rust_callee(node, source, ctx, next_function.as_ref()) {
                callees.push(callee);
            }
        }
        if node.kind() == "method_call_expr" {
            if let Some(callee) = extract_rust_callee(node, source, ctx, next_function.as_ref()) {
                callees.push(callee);
            }
        }
    }

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
    current_function: Option<&RustFunctionContext>,
) -> Option<CalleeDefinition> {
    let (callee_name, receiver_type) = match node.kind() {
        "call_expression" => {
            let func = node.child_by_field_name("function")?;
            match func.kind() {
                "identifier" => {
                    let name = rust_node_text(func, source)?;
                    let qualified = qualify_rust_function_name(node, &name, source).unwrap_or(name);
                    (qualified, None)
                }
                "scoped_identifier" => {
                    let name = rust_node_text(func, source)?;
                    let qualified = qualify_scoped_rust_call_target(node, &name, source);
                    (qualified, None)
                }
                "field_expression" => {
                    let receiver = func
                        .child_by_field_name("value")
                        .and_then(|n| rust_node_text(n, source));
                    let method = func
                        .child_by_field_name("field")
                        .and_then(|n| rust_node_text(n, source))?;
                    let qualified = receiver
                        .as_deref()
                        .and_then(|name| qualify_receiver_method(name, &method, current_function))
                        .unwrap_or(method);
                    (qualified, receiver)
                }
                _ => {
                    let name = rust_node_text(func, source)?;
                    (name, None)
                }
            }
        }
        "method_call_expr" => {
            let receiver = node
                .child_by_field_name("receiver")
                .and_then(|n| rust_node_text(n, source));
            let method = node
                .child_by_field_name("method")
                .and_then(|n| rust_node_text(n, source))
                .or_else(|| {
                    node.named_children(&mut node.walk())
                        .find(|child| child.kind() == "field_identifier")
                        .and_then(|child| rust_node_text(child, source))
                })?;
            let qualified = receiver
                .as_deref()
                .and_then(|name| qualify_receiver_method(name, &method, current_function))
                .unwrap_or(method);
            (qualified, receiver)
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
    node_text(node, source)
}

fn qualify_rust_function_name(node: Node, base_name: &str, source: &[u8]) -> Option<String> {
    let owner = enclosing_impl_owner(node, source)?;
    Some(format!("{}::{}", owner, base_name))
}

fn enclosing_impl_owner(node: Node, source: &[u8]) -> Option<String> {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == "impl_item" {
            return extract_impl_owner_name(parent, source);
        }
        current = parent.parent();
    }
    None
}

fn extract_impl_owner_name(impl_item: Node, source: &[u8]) -> Option<String> {
    if let Some(type_node) = impl_item.child_by_field_name("type") {
        return rust_node_text(type_node, source)
            .map(|value| simplify_rust_type_name(&value))
            .filter(|value| !value.is_empty());
    }

    for index in 0..impl_item.named_child_count() {
        let child = impl_item.named_child(index)?;
        if matches!(
            child.kind(),
            "type_identifier"
                | "scoped_type_identifier"
                | "generic_type"
                | "tuple_type"
                | "reference_type"
                | "primitive_type"
        ) {
            if let Some(value) = rust_node_text(child, source) {
                let simplified = simplify_rust_type_name(&value);
                if !simplified.is_empty() {
                    return Some(simplified);
                }
            }
        }
    }

    None
}

fn simplify_rust_type_name(value: &str) -> String {
    let trimmed = value.trim();
    let without_ref = trimmed
        .trim_start_matches('&')
        .trim_start_matches("mut ")
        .trim();
    let base = without_ref.split('<').next().unwrap_or(without_ref).trim();
    base.rsplit("::").next().unwrap_or(base).trim().to_string()
}

fn qualify_scoped_rust_call_target(node: Node, value: &str, source: &[u8]) -> String {
    let trimmed = value.trim();
    if let Some(stripped) = trimmed.strip_prefix("Self::") {
        if let Some(owner) = enclosing_impl_owner(node, source) {
            return format!("{}::{}", owner, stripped);
        }
        return stripped.to_string();
    }
    trimmed.to_string()
}

fn collect_rust_local_bindings(
    function_node: Node,
    source: &[u8],
    owner_name: Option<&str>,
) -> HashMap<String, RustBindingMeta> {
    let mut bindings = HashMap::new();
    collect_rust_local_bindings_recursive(function_node, source, owner_name, &mut bindings);
    bindings
}

fn collect_rust_local_bindings_recursive(
    node: Node,
    source: &[u8],
    owner_name: Option<&str>,
    bindings: &mut HashMap<String, RustBindingMeta>,
) {
    if node.kind() == "let_declaration" {
        if let Some((binding_name, binding_meta)) = extract_rust_binding(node, source, owner_name) {
            bindings.insert(binding_name, binding_meta);
        }
    }

    for index in 0..node.named_child_count() {
        if let Some(child) = node.named_child(index) {
            collect_rust_local_bindings_recursive(child, source, owner_name, bindings);
        }
    }
}

fn extract_rust_binding(
    node: Node,
    source: &[u8],
    owner_name: Option<&str>,
) -> Option<(String, RustBindingMeta)> {
    let mut identifier_node = None;
    let mut value_node = None;
    for index in 0..node.named_child_count() {
        let Some(child) = node.named_child(index) else {
            continue;
        };
        match child.kind() {
            "identifier" if identifier_node.is_none() => identifier_node = Some(child),
            "call_expression" if value_node.is_none() => value_node = Some(child),
            _ => {}
        }
    }

    let name_node = identifier_node?;
    let value_node = value_node?;
    let binding_name = rust_node_text(name_node, source)?;
    if binding_name.contains(['{', '[', '(', ')']) {
        return None;
    }

    // Only bind when the initializer clearly names a constructor/factory owner.
    let owner_type = extract_rust_call_target(value_node, source).and_then(|name| {
        if name.strip_prefix("Self::").is_some() {
            return owner_name.map(str::to_string);
        }

        name.rsplit_once("::").map(|(owner, _)| owner.to_string())
    })?;

    Some((binding_name, RustBindingMeta { owner_type }))
}

fn qualify_receiver_method(
    receiver_name: &str,
    method_name: &str,
    current_function: Option<&RustFunctionContext>,
) -> Option<String> {
    let context = current_function?;
    let owner = context.local_bindings.get(receiver_name)?;
    Some(format!("{}::{}", owner.owner_type(), method_name))
}

fn extract_rust_call_target(node: Node, source: &[u8]) -> Option<String> {
    if node.kind() != "call_expression" {
        return None;
    }

    for index in 0..node.named_child_count() {
        let Some(child) = node.named_child(index) else {
            continue;
        };
        match child.kind() {
            "identifier" | "scoped_identifier" | "field_expression" => {
                return rust_node_text(child, source);
            }
            _ => {}
        }
    }

    None
}
