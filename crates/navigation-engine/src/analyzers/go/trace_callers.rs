use std::path::Path;

use tree_sitter::{Node, Parser};

use super::super::types::{
    infer_public_language, CallerDefinition, CallerTarget, FindCallersQuery,
};
use super::common::{
    extract_go_file_context, extract_go_function_context, go_module_name, go_module_root,
    go_path_matches_target, node_text, normalize_go_target_symbol, resolve_go_call_target,
    GoFileContext, GoFunctionContext, GoTargetSymbol,
};

struct CallerContext<'a> {
    target: GoTargetSymbol,
    current_file: &'a Path,
    target_path: &'a Path,
    public_language: Option<&'a str>,
    file_context: GoFileContext,
    same_file_target: bool,
}

pub(super) fn find_callers(
    _workspace_root: &Path,
    path: &Path,
    source: &str,
    query: &FindCallersQuery,
) -> Vec<CallerDefinition> {
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
    let public_language = infer_public_language(path);
    let context = CallerContext {
        target: normalize_go_target_symbol(&query.target_symbol),
        current_file: path,
        target_path: query.target_path.as_path(),
        public_language: public_language.as_deref(),
        file_context: extract_go_file_context(
            tree.root_node(),
            source.as_bytes(),
            workspace_root.as_deref(),
            module_name.as_deref(),
        ),
        same_file_target: go_path_matches_target(path, &query.target_path),
    };

    let mut callers = Vec::new();
    collect_go_callers(
        tree.root_node(),
        source.as_bytes(),
        None,
        &context,
        &mut callers,
    );
    callers.sort_by(|left, right| {
        (
            &left.path,
            left.caller_symbol
                .as_deref()
                .unwrap_or(left.caller.as_str()),
            left.line,
            left.column.unwrap_or(0),
        )
            .cmp(&(
                &right.path,
                right
                    .caller_symbol
                    .as_deref()
                    .unwrap_or(right.caller.as_str()),
                right.line,
                right.column.unwrap_or(0),
            ))
    });
    callers.dedup_by(|left, right| {
        left.path == right.path
            && left.caller_symbol == right.caller_symbol
            && left.caller == right.caller
            && left.calls == right.calls
    });
    callers
}

fn collect_go_callers(
    node: Node,
    source: &[u8],
    current_function: Option<GoFunctionContext>,
    ctx: &CallerContext,
    callers: &mut Vec<CallerDefinition>,
) {
    let next_function = extract_go_function_context(node, source).or(current_function.clone());

    if let Some(function_context) = &next_function {
        if node.kind() == "call_expression" {
            if let Some(caller) = extract_go_caller(node, source, function_context, ctx) {
                callers.push(caller);
            }
        }
    }

    for index in 0..node.named_child_count() {
        let Some(child) = node.named_child(index) else {
            continue;
        };
        collect_go_callers(child, source, next_function.clone(), ctx, callers);
    }
}

fn extract_go_caller(
    node: Node,
    source: &[u8],
    function_context: &GoFunctionContext,
    ctx: &CallerContext,
) -> Option<CallerDefinition> {
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

    if !ctx
        .target
        .matches(&resolved.symbol, &resolved.destination, ctx.target_path)
    {
        return None;
    }

    if ctx.same_file_target && function_context.symbol == ctx.target.display() {
        return None;
    }

    Some(CallerDefinition {
        path: ctx.current_file.to_string_lossy().replace('\\', "/"),
        line: (node.start_position().row + 1) as u32,
        column: Some((node.start_position().column + 1) as u32),
        caller: function_context.symbol.clone(),
        caller_symbol: Some(function_context.symbol.clone()),
        relation: "calls".to_string(),
        language: ctx.public_language.map(str::to_string),
        snippet: node_text(node, source),
        receiver_type: resolved.receiver_type,
        calls: CallerTarget {
            path: ctx.target_path.to_string_lossy().replace('\\', "/"),
            symbol: ctx.target.display(),
        },
        probable_entry_point_reasons: Vec::new(),
    })
}
