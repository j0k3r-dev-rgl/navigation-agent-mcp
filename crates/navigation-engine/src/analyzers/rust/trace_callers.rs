use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use tree_sitter::{Node, Parser};

use super::super::types::{
    infer_public_language, CallerCallSite, CallerDefinition, CallerRange, CallerTarget,
    FindCallersQuery,
};
use super::common::node_text;

struct CallerContext<'a> {
    target: TargetSymbol<'a>,
    current_file: &'a Path,
    target_path: &'a Path,
    public_language: Option<&'a str>,
    same_file_target: bool,
}

struct TargetSymbol<'a> {
    full: &'a str,
    owner: Option<&'a str>,
    base: &'a str,
}

#[derive(Clone)]
struct FunctionContext {
    caller: String,
    caller_symbol: Option<String>,
    owner_name: Option<String>,
    local_bindings: HashMap<String, String>,
    probable_entry_point_reasons: Vec<String>,
    caller_range: CallerRange,
}

struct ResolvedCall {
    symbol: String,
    receiver_type: Option<String>,
}

pub(super) fn find_callers(
    _workspace_root: &Path,
    path: &Path,
    source: &str,
    query: &FindCallersQuery,
) -> Vec<CallerDefinition> {
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
    let context = CallerContext {
        target: TargetSymbol::new(query.target_symbol.as_str()),
        current_file: path,
        target_path: query.target_path.as_path(),
        public_language: public_language.as_deref(),
        same_file_target: normalize_path(path) == normalize_path(&query.target_path),
    };

    let mut callers = Vec::new();
    walk_for_callers(
        tree.root_node(),
        source.as_bytes(),
        None,
        &context,
        &mut callers,
    );
    dedupe_callers(callers)
}

impl<'a> TargetSymbol<'a> {
    fn new(value: &'a str) -> Self {
        if let Some((owner, base)) = value.rsplit_once("::") {
            Self {
                full: value,
                owner: Some(owner),
                base,
            }
        } else {
            Self {
                full: value,
                owner: None,
                base: value,
            }
        }
    }

    fn matches(&self, candidate: &str) -> bool {
        candidate == self.full
            || self
                .owner
                .is_some_and(|_| trailing_segments(candidate, 2) == Some(self.full))
            || (self.owner.is_none()
                && (candidate == self.base || trailing_segments(candidate, 1) == Some(self.base)))
    }
}

fn walk_for_callers(
    node: Node,
    source: &[u8],
    current_context: Option<FunctionContext>,
    ctx: &CallerContext,
    callers: &mut Vec<CallerDefinition>,
) {
    let next_context = derive_function_context(node, source).or(current_context.clone());

    if let Some(function_context) = &next_context {
        if let Some(caller) = extract_call_reference(node, source, function_context, ctx) {
            callers.push(caller);
        }
    }

    for index in 0..node.named_child_count() {
        let Some(child) = node.named_child(index) else {
            continue;
        };
        walk_for_callers(child, source, next_context.clone(), ctx, callers);
    }
}

fn derive_function_context(node: Node, source: &[u8]) -> Option<FunctionContext> {
    if node.kind() != "function_item" {
        return None;
    }

    let name = node
        .child_by_field_name("name")
        .and_then(|item| node_text(item, source))?;
    let owner_name = enclosing_impl_owner(node, source);
    let caller_symbol = owner_name
        .as_ref()
        .map(|owner| format!("{}::{}", owner, name))
        .or_else(|| Some(name.clone()));
    let probable_entry_point_reasons = extract_probable_entry_point_reasons(node, source);
    let local_bindings = collect_rust_local_bindings(node, source, owner_name.as_deref());

    Some(FunctionContext {
        caller: caller_symbol.clone().unwrap_or_else(|| name.clone()),
        caller_symbol,
        owner_name,
        local_bindings,
        probable_entry_point_reasons,
        caller_range: CallerRange {
            start_line: (node.start_position().row + 1) as u32,
            end_line: (node.end_position().row + 1) as u32,
        },
    })
}

fn extract_call_reference(
    node: Node,
    source: &[u8],
    function_context: &FunctionContext,
    ctx: &CallerContext,
) -> Option<CallerDefinition> {
    let call_target = extract_call_target(node, source, function_context)?;
    let receiver_type = call_target.receiver_type.clone();
    if !ctx.target.matches(call_target.symbol.as_str()) {
        return None;
    }

    if ctx.same_file_target
        && function_context
            .caller_symbol
            .as_deref()
            .is_some_and(|symbol| ctx.target.matches(symbol))
    {
        return None;
    }

    Some(CallerDefinition {
        path: ctx.current_file.to_string_lossy().replace('\\', "/"),
        line: (node.start_position().row + 1) as u32,
        column: Some((node.start_position().column + 1) as u32),
        caller: function_context.caller.clone(),
        caller_symbol: function_context.caller_symbol.clone(),
        relation: "calls".to_string(),
        language: ctx.public_language.map(str::to_string),
        snippet: node_text(node, source),
        receiver_type: receiver_type.clone(),
        caller_range: function_context.caller_range.clone(),
        call_site: CallerCallSite {
            line: (node.start_position().row + 1) as u32,
            column: Some((node.start_position().column + 1) as u32),
            relation: "calls".to_string(),
            snippet: node_text(node, source),
            receiver_type,
        },
        calls: CallerTarget {
            path: ctx.target_path.to_string_lossy().replace('\\', "/"),
            symbol: ctx.target.full.to_string(),
        },
        probable_entry_point_reasons: function_context.probable_entry_point_reasons.clone(),
    })
}

fn extract_call_target(
    node: Node,
    source: &[u8],
    function_context: &FunctionContext,
) -> Option<ResolvedCall> {
    match node.kind() {
        "call_expression" => {
            let function = node.child_by_field_name("function")?;
            match function.kind() {
                "field_expression" => {
                    let receiver = function
                        .child_by_field_name("value")
                        .and_then(|item| node_text(item, source))
                        .or_else(|| {
                            function
                                .named_child(0)
                                .and_then(|item| node_text(item, source))
                        });
                    let method = function
                        .child_by_field_name("field")
                        .and_then(|item| node_text(item, source))
                        .or_else(|| {
                            function
                                .named_children(&mut function.walk())
                                .find(|child| child.kind() == "field_identifier")
                                .and_then(|child| node_text(child, source))
                        })?;
                    let symbol = receiver
                        .as_deref()
                        .and_then(|value| qualify_receiver_method(value, &method, function_context))
                        .unwrap_or(method);
                    Some(ResolvedCall {
                        symbol,
                        receiver_type: receiver,
                    })
                }
                "scoped_identifier" => {
                    let raw = node_text(function, source)?;
                    Some(ResolvedCall {
                        symbol: qualify_scoped_target(node, &raw, source),
                        receiver_type: None,
                    })
                }
                _ => {
                    let raw = node_text(function, source)?;
                    Some(ResolvedCall {
                        symbol: raw,
                        receiver_type: None,
                    })
                }
            }
        }
        "method_call_expr" => {
            let receiver = node
                .child_by_field_name("receiver")
                .and_then(|item| node_text(item, source));
            let method = node
                .child_by_field_name("method")
                .and_then(|item| node_text(item, source))
                .or_else(|| {
                    node.named_children(&mut node.walk())
                        .find(|child| child.kind() == "field_identifier")
                        .and_then(|child| node_text(child, source))
                })?;
            let symbol = receiver
                .as_deref()
                .and_then(|value| qualify_receiver_method(value, &method, function_context))
                .unwrap_or(method);
            Some(ResolvedCall {
                symbol,
                receiver_type: receiver,
            })
        }
        _ => None,
    }
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn dedupe_callers(callers: Vec<CallerDefinition>) -> Vec<CallerDefinition> {
    let mut unique = BTreeMap::new();
    for caller in callers {
        unique.insert(
            (
                caller.path.clone(),
                caller.line,
                caller.column.unwrap_or(0),
                caller.caller.clone(),
                caller.caller_symbol.clone().unwrap_or_default(),
                caller.relation.clone(),
            ),
            caller,
        );
    }
    unique.into_values().collect()
}

fn trailing_segments<'a>(value: &'a str, count: usize) -> Option<&'a str> {
    let indices = value
        .match_indices("::")
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if count <= 1 {
        return indices
            .last()
            .map(|index| &value[index + 2..])
            .or(Some(value));
    }

    if indices.len() + 1 < count {
        return None;
    }

    let split_index = indices[indices.len() - (count - 1)];
    Some(&value[split_index + 2..])
}

fn qualify_scoped_target(node: Node, value: &str, source: &[u8]) -> String {
    let trimmed = value.trim();
    if let Some(stripped) = trimmed.strip_prefix("Self::") {
        if let Some(owner) = enclosing_impl_owner(node, source) {
            return format!("{}::{}", owner, stripped);
        }
        return stripped.to_string();
    }

    trailing_segments(trimmed, 2).unwrap_or(trimmed).to_string()
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
        return node_text(type_node, source)
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
            if let Some(value) = node_text(child, source) {
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

fn collect_rust_local_bindings(
    function_node: Node,
    source: &[u8],
    owner_name: Option<&str>,
) -> HashMap<String, String> {
    let mut bindings = HashMap::new();
    collect_rust_local_bindings_recursive(function_node, source, owner_name, &mut bindings);
    bindings
}

fn collect_rust_local_bindings_recursive(
    node: Node,
    source: &[u8],
    owner_name: Option<&str>,
    bindings: &mut HashMap<String, String>,
) {
    if node.kind() == "let_declaration" {
        if let Some((binding_name, resolved_type)) = extract_rust_binding(node, source, owner_name)
        {
            bindings.insert(binding_name, resolved_type);
        }
    }

    for index in 0..node.named_child_count() {
        let Some(child) = node.named_child(index) else {
            continue;
        };
        collect_rust_local_bindings_recursive(child, source, owner_name, bindings);
    }
}

fn extract_rust_binding(
    node: Node,
    source: &[u8],
    owner_name: Option<&str>,
) -> Option<(String, String)> {
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

    let binding_name = node_text(identifier_node?, source)?;
    if binding_name.contains(['{', '[', '(', ')']) {
        return None;
    }

    let resolved_type = extract_rust_call_target(value_node?, source).and_then(|name| {
        if name.strip_prefix("Self::").is_some() {
            return owner_name.map(str::to_string);
        }

        if let Some((owner, _)) = name.rsplit_once("::") {
            return Some(owner.to_string());
        }

        Some(name)
    })?;

    Some((binding_name, resolved_type))
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
                return node_text(child, source);
            }
            _ => {}
        }
    }

    None
}

fn qualify_receiver_method(
    receiver_name: &str,
    method_name: &str,
    function_context: &FunctionContext,
) -> Option<String> {
    if receiver_name == "self" {
        let owner = function_context.owner_name.as_deref()?;
        return Some(format!("{}::{}", owner, method_name));
    }

    let owner = function_context.local_bindings.get(receiver_name)?;
    let owner_name = owner.rsplit("::").next().unwrap_or(owner);
    Some(format!("{}::{}", owner_name, method_name))
}

fn extract_probable_entry_point_reasons(node: Node, source: &[u8]) -> Vec<String> {
    let mut reasons = Vec::new();

    for attribute in leading_attribute_items(node) {
        let Some(text) = node_text(attribute, source) else {
            continue;
        };
        let Some(inner) = text
            .strip_prefix("#[")
            .and_then(|value| value.strip_suffix(']'))
            .map(str::trim)
        else {
            continue;
        };
        let macro_name = inner
            .split_once('(')
            .map(|(name, _)| name)
            .unwrap_or(inner)
            .rsplit("::")
            .next()
            .unwrap_or(inner)
            .trim();
        if matches!(macro_name, "get" | "post" | "put" | "delete" | "patch") {
            reasons.push("public rest handler".to_string());
            break;
        }
    }

    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == "impl_item" {
            if graphql_impl_kind(parent, source).is_some() {
                reasons.push("public graphql method".to_string());
            }
            break;
        }
        current = parent.parent();
    }

    reasons
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

fn graphql_impl_kind(impl_item: Node, source: &[u8]) -> Option<&'static str> {
    for attribute in leading_attribute_items(impl_item) {
        let Some(text) = node_text(attribute, source) else {
            continue;
        };
        let Some(inner) = text
            .strip_prefix("#[")
            .and_then(|value| value.strip_suffix(']'))
            .map(str::trim)
        else {
            continue;
        };
        let macro_name = inner
            .split_once('(')
            .map(|(name, _)| name)
            .unwrap_or(inner)
            .rsplit("::")
            .next()
            .unwrap_or(inner)
            .trim();

        match macro_name {
            "Object" => return Some("Object"),
            "Subscription" => return Some("Subscription"),
            _ => continue,
        }
    }

    None
}
