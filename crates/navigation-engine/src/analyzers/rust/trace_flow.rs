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
struct RustFunctionContext;

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
            .map(|name| name == ctx.target_symbol)
            .unwrap_or(false);

    let next_function = if is_target_function || current_function.is_some() {
        Some(RustFunctionContext)
    } else {
        current_function.clone()
    };

    if is_target_function || current_function.is_some() {
        if node.kind() == "call_expression" {
            if let Some(callee) = extract_rust_callee(node, source, ctx) {
                callees.push(callee);
            }
        }
        if node.kind() == "method_call_expr" {
            if let Some(callee) = extract_rust_callee(node, source, ctx) {
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
    node_text(node, source)
}
