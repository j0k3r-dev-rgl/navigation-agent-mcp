use std::path::Path;

use tree_sitter::{Node, Parser};

use super::super::types::{infer_public_language, CalleeDefinition, FindCalleesQuery};
use super::common::{
    extract_go_file_context, extract_go_function_context, go_module_name, go_module_root,
    go_symbol_matches_target, node_text, resolve_go_call_target, GoFileContext, GoFunctionContext,
};

pub(super) fn find_callees(
    path: &Path,
    source: &str,
    query: &FindCalleesQuery,
) -> Vec<CalleeDefinition> {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_go::LANGUAGE.into())
        .is_err()
    {
        return Vec::new();
    }

    let Some(tree) = parser.parse(source, None) else {
        return Vec::new();
    };

    let workspace_root = go_module_root(path.parent().unwrap_or(path));
    let module_name = workspace_root
        .as_ref()
        .and_then(|root| go_module_name(root));
    let file_ctx = extract_go_file_context(
        tree.root_node(),
        source.as_bytes(),
        workspace_root.as_deref(),
        module_name.as_deref(),
    );
    let public_language = infer_public_language(path);
    let mut callees = Vec::new();

    collect_go_callees(
        tree.root_node(),
        source.as_bytes(),
        None,
        &GoCalleeContext {
            target_symbol: &query.target_symbol,
            current_file: path,
            public_language: public_language.as_deref(),
            file_context: file_ctx,
        },
        &mut callees,
    );

    callees
}

struct GoCalleeContext<'a> {
    target_symbol: &'a str,
    current_file: &'a Path,
    public_language: Option<&'a str>,
    file_context: GoFileContext,
}

fn collect_go_callees(
    node: Node,
    source: &[u8],
    current_function: Option<GoFunctionContext>,
    ctx: &GoCalleeContext,
    callees: &mut Vec<CalleeDefinition>,
) {
    let is_target = matches!(node.kind(), "function_declaration" | "method_declaration")
        && super::common::current_go_symbol(node, source)
            .map(|symbol| go_symbol_matches_target(&symbol, ctx.target_symbol))
            .unwrap_or(false);

    let next_function = if is_target {
        extract_go_function_context(node, source)
    } else {
        current_function.clone()
    };

    if let Some(function_context) = &next_function {
        if node.kind() == "call_expression" {
            if let Some(callee) = extract_go_callee(node, source, function_context, ctx) {
                callees.push(callee);
            }
        }
    }

    for index in 0..node.named_child_count() {
        if let Some(child) = node.named_child(index) {
            collect_go_callees(child, source, next_function.clone(), ctx, callees);
        }
    }
}

fn extract_go_callee(
    node: Node,
    source: &[u8],
    function_context: &GoFunctionContext,
    ctx: &GoCalleeContext,
) -> Option<CalleeDefinition> {
    let function = node
        .child_by_field_name("function")
        .or_else(|| node.named_child(0))?;
    let resolved = resolve_go_call_target(
        function,
        source,
        &ctx.file_context,
        ctx.current_file,
        function_context,
    )?;

    Some(CalleeDefinition {
        path: resolved.destination.to_string_lossy().replace('\\', "/"),
        line: (node.start_position().row + 1) as u32,
        end_line: (node.end_position().row + 1) as u32,
        column: Some((node.start_position().column + 1) as u32),
        callee: resolved.symbol,
        callee_symbol: Some(function_context.symbol.clone()),
        receiver_type: resolved.receiver_type,
        relation: "calls".to_string(),
        language: ctx.public_language.map(str::to_string),
        snippet: node_text(node, source),
    })
}
