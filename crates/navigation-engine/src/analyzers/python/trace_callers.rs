use std::path::Path;

use tree_sitter::{Node, Parser};

use super::super::types::{
    infer_public_language, CallerCallSite, CallerDefinition, CallerRange, CallerTarget,
    FindCallersQuery,
};
use super::common::node_text;

struct PythonCallerContext<'a> {
    target_symbol: &'a str,
    target_path: &'a Path,
    current_file: &'a Path,
    public_language: Option<&'a str>,
}

#[derive(Clone)]
struct PythonFunctionContext {
    symbol: String,
    start_line: u32,
    end_line: u32,
}

pub(super) fn find_callers(
    _workspace_root: &Path,
    path: &Path,
    source: &str,
    query: &FindCallersQuery,
) -> Vec<CallerDefinition> {
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
    let ctx = PythonCallerContext {
        target_symbol: &query.target_symbol,
        target_path: &query.target_path,
        current_file: path,
        public_language: public_language.as_deref(),
    };

    let mut callers = Vec::new();
    collect_python_callers(
        tree.root_node(),
        source.as_bytes(),
        None,
        &ctx,
        &mut callers,
    );

    callers
}

fn collect_python_callers(
    node: Node,
    source: &[u8],
    current_function: Option<PythonFunctionContext>,
    ctx: &PythonCallerContext,
    callers: &mut Vec<CallerDefinition>,
) {
    let next_function = match node.kind() {
        "function_definition" | "async_function_definition" => {
            let name_node = node.child_by_field_name("name");
            let symbol = name_node
                .and_then(|n| node_text(n, source))
                .unwrap_or_else(|| "<anonymous>".to_string());
            Some(PythonFunctionContext {
                symbol,
                start_line: (node.start_position().row + 1) as u32,
                end_line: (node.end_position().row + 1) as u32,
            })
        }
        _ => current_function.clone(),
    };

    if let Some(function_ctx) = &next_function {
        if node.kind() == "call" {
            if let Some(caller) = extract_python_caller(node, source, function_ctx, ctx) {
                // Avoid self-calls if we're in the target file and function
                let is_self_call =
                    ctx.current_file == ctx.target_path && function_ctx.symbol == ctx.target_symbol;
                if !is_self_call {
                    callers.push(caller);
                }
            }
        }
    }

    for index in 0..node.named_child_count() {
        if let Some(child) = node.named_child(index) {
            collect_python_callers(child, source, next_function.clone(), ctx, callers);
        }
    }
}

fn extract_python_caller(
    node: Node,
    source: &[u8],
    function_ctx: &PythonFunctionContext,
    ctx: &PythonCallerContext,
) -> Option<CallerDefinition> {
    let function_node = node.child_by_field_name("function")?;

    let (callee_name, receiver_type) = match function_node.kind() {
        "identifier" => {
            let name = node_text(function_node, source)?;
            (name, None)
        }
        "attribute" => {
            let object = function_node
                .child_by_field_name("object")
                .and_then(|n| node_text(n, source));
            let attr = function_node
                .child_by_field_name("attribute")
                .and_then(|n| node_text(n, source));
            let name = attr.unwrap_or_else(|| node_text(function_node, source).unwrap_or_default());
            (name, object)
        }
        _ => return None,
    };

    if callee_name != ctx.target_symbol {
        return None;
    }

    // In a real implementation, we would check if receiver_type matches the target class/module.
    // For now, we match by symbol name, which is a good baseline for Python impact analysis.

    Some(CallerDefinition {
        path: ctx.current_file.to_string_lossy().replace('\\', "/"),
        line: (node.start_position().row + 1) as u32,
        column: Some((node.start_position().column + 1) as u32),
        caller: function_ctx.symbol.clone(),
        caller_symbol: Some(function_ctx.symbol.clone()),
        relation: "calls".to_string(),
        language: ctx.public_language.map(String::from),
        snippet: node_text(node, source),
        receiver_type: receiver_type.clone(),
        caller_range: CallerRange {
            start_line: function_ctx.start_line,
            end_line: function_ctx.end_line,
        },
        call_site: CallerCallSite {
            line: (node.start_position().row + 1) as u32,
            column: Some((node.start_position().column + 1) as u32),
            relation: "calls".to_string(),
            snippet: node_text(node, source),
            receiver_type,
        },
        calls: CallerTarget {
            path: ctx.target_path.to_string_lossy().replace('\\', "/"),
            symbol: ctx.target_symbol.to_string(),
        },
        probable_entry_point_reasons: Vec::new(),
    })
}
