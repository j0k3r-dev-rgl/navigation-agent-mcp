use std::path::Path;

use tree_sitter::{Node, Parser};

use super::super::types::{infer_public_language, CalleeDefinition, FindCalleesQuery};
use super::common::node_text;

pub(super) struct PythonCalleeContext<'a> {
    pub(super) target_symbol: &'a str,
    pub(super) current_file: &'a Path,
    pub(super) public_language: Option<&'a str>,
}

#[derive(Clone)]
struct PythonFunctionContext;

pub(super) fn find_callees(
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

fn collect_python_callees(
    node: Node,
    source: &[u8],
    current_function: Option<PythonFunctionContext>,
    ctx: &PythonCalleeContext,
    callees: &mut Vec<CalleeDefinition>,
) {
    let is_target_function = match node.kind() {
        "function_definition" | "async_function_definition" => node
            .child_by_field_name("name")
            .and_then(|n| python_node_text(n, source))
            .map(|name| name == ctx.target_symbol)
            .unwrap_or(false),
        _ => false,
    };

    let next_function = if is_target_function || current_function.is_some() {
        Some(PythonFunctionContext)
    } else {
        current_function.clone()
    };

    if is_target_function || current_function.is_some() {
        if node.kind() == "call" {
            if let Some(callee) = extract_python_callee(node, source, ctx) {
                callees.push(callee);
            }
        }
    }

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
    node_text(node, source)
}
