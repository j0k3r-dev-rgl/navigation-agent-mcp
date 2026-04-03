use std::collections::{HashMap, HashSet};
use std::path::Path;

use tree_sitter::{Node, Parser};

use super::super::types::{
    infer_public_language, CallerCallSite, CallerDefinition, CallerRange, CallerTarget,
    FindCallersQuery,
};
use super::common::{
    extract_go_file_context, extract_go_function_context_with_file_context, go_module_name,
    go_module_root, go_path_matches_target, node_text, normalize_go_target_symbol,
    resolve_go_call_target, GoFileContext, GoFunctionContext, GoTargetSymbol,
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
    let next_function = extract_go_function_context_with_file_context(
        node,
        source,
        &ctx.file_context,
        ctx.current_file,
    )
    .or(current_function.clone());

    if let Some(function_context) = &next_function {
        if node.kind() == "call_expression" {
            if let Some(caller) = extract_go_caller(node, source, function_context, ctx) {
                callers.push(caller);
            }
        }
        if node.kind() == "selector_expression" {
            if let Some(caller) =
                extract_go_method_value_caller(node, source, function_context, ctx)
            {
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
    let receiver_type = resolved.receiver_type.clone();

    if !resolved_matches_target(ctx, &resolved.symbol, &resolved.destination) {
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
        receiver_type: receiver_type.clone(),
        caller_range: CallerRange {
            start_line: function_context.start_line,
            end_line: function_context.end_line,
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
            symbol: ctx.target.display(),
        },
        probable_entry_point_reasons: Vec::new(),
    })
}

fn extract_go_method_value_caller(
    node: Node,
    source: &[u8],
    function_context: &GoFunctionContext,
    ctx: &CallerContext,
) -> Option<CallerDefinition> {
    if let Some(parent) = node.parent() {
        if parent.kind() == "call_expression" {
            let function_node = parent
                .child_by_field_name("function")
                .or_else(|| parent.named_child(0));
            if function_node.is_some_and(|function| function.id() == node.id()) {
                return None;
            }
        }
    }

    let resolved = resolve_go_call_target(
        node,
        source,
        &ctx.file_context,
        ctx.current_file,
        function_context,
    )?;
    let receiver_type = resolved.receiver_type.clone();

    if !resolved_matches_target(ctx, &resolved.symbol, &resolved.destination) {
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
        relation: "references".to_string(),
        language: ctx.public_language.map(str::to_string),
        snippet: node_text(node, source),
        receiver_type: receiver_type.clone(),
        caller_range: CallerRange {
            start_line: function_context.start_line,
            end_line: function_context.end_line,
        },
        call_site: CallerCallSite {
            line: (node.start_position().row + 1) as u32,
            column: Some((node.start_position().column + 1) as u32),
            relation: "references".to_string(),
            snippet: node_text(node, source),
            receiver_type,
        },
        calls: CallerTarget {
            path: ctx.target_path.to_string_lossy().replace('\\', "/"),
            symbol: ctx.target.display(),
        },
        probable_entry_point_reasons: Vec::new(),
    })
}

fn resolved_matches_target(ctx: &CallerContext, symbol: &str, destination: &Path) -> bool {
    if ctx.target.matches(symbol, destination, ctx.target_path) {
        return true;
    }

    let Some((target_owner, target_method)) = ctx.target.method_parts() else {
        return false;
    };
    let Some((resolved_owner, resolved_method)) = symbol.rsplit_once('.') else {
        return false;
    };

    if resolved_method != target_method {
        return false;
    }

    interface_receiver_matches_target_implementation(
        destination,
        ctx.target_path,
        resolved_owner,
        target_owner,
        target_method,
    )
}

fn interface_receiver_matches_target_implementation(
    destination: &Path,
    target_path: &Path,
    interface_owner: &str,
    concrete_owner: &str,
    method_name: &str,
) -> bool {
    let interface_dir = if destination.is_dir() {
        destination.to_path_buf()
    } else {
        destination.parent().unwrap_or(destination).to_path_buf()
    };
    let target_dir = target_path.parent().unwrap_or(target_path);
    if interface_dir != target_dir {
        return false;
    }

    let Ok(package_types) = collect_go_package_types(target_dir) else {
        return false;
    };

    package_types
        .interface_methods
        .get(interface_owner)
        .is_some_and(|methods| methods.contains(method_name))
        && package_types
            .concrete_methods
            .get(concrete_owner)
            .is_some_and(|methods| methods.contains(method_name))
}

struct GoPackageTypes {
    interface_methods: HashMap<String, HashSet<String>>,
    concrete_methods: HashMap<String, HashSet<String>>,
}

fn collect_go_package_types(dir: &Path) -> Result<GoPackageTypes, ()> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_go::LANGUAGE.into())
        .map_err(|_| ())?;

    let mut interface_methods = HashMap::<String, HashSet<String>>::new();
    let mut concrete_methods = HashMap::<String, HashSet<String>>::new();

    let entries = std::fs::read_dir(dir).map_err(|_| ())?;
    for entry in entries {
        let entry = entry.map_err(|_| ())?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("go") {
            continue;
        }
        let source = std::fs::read_to_string(&path).map_err(|_| ())?;
        let Some(tree) = parser.parse(&source, None) else {
            continue;
        };
        collect_types_from_node(
            tree.root_node(),
            source.as_bytes(),
            &mut interface_methods,
            &mut concrete_methods,
        );
    }

    Ok(GoPackageTypes {
        interface_methods,
        concrete_methods,
    })
}

fn collect_types_from_node(
    node: Node,
    source: &[u8],
    interface_methods: &mut HashMap<String, HashSet<String>>,
    concrete_methods: &mut HashMap<String, HashSet<String>>,
) {
    match node.kind() {
        "type_spec" => collect_interface_methods(node, source, interface_methods),
        "method_declaration" => collect_concrete_method(node, source, concrete_methods),
        _ => {}
    }

    for index in 0..node.named_child_count() {
        if let Some(child) = node.named_child(index) {
            collect_types_from_node(child, source, interface_methods, concrete_methods);
        }
    }
}

fn collect_interface_methods(
    node: Node,
    source: &[u8],
    interface_methods: &mut HashMap<String, HashSet<String>>,
) {
    let Some(name_node) = node.child_by_field_name("name") else {
        return;
    };
    let Some(type_node) = node.child_by_field_name("type") else {
        return;
    };
    if type_node.kind() != "interface_type" {
        return;
    }
    let Some(interface_name) = node_text(name_node, source) else {
        return;
    };

    let entry = interface_methods.entry(interface_name).or_default();
    for index in 0..type_node.named_child_count() {
        let Some(method_elem) = type_node.named_child(index) else {
            continue;
        };
        if method_elem.kind() != "method_elem" {
            continue;
        }
        if let Some(method_name) = method_elem
            .child_by_field_name("name")
            .and_then(|name| node_text(name, source))
            .or_else(|| {
                (0..method_elem.named_child_count())
                    .filter_map(|idx| method_elem.named_child(idx))
                    .find(|n| n.kind() == "field_identifier")
                    .and_then(|n| node_text(n, source))
            })
        {
            entry.insert(method_name);
        }
    }
}

fn collect_concrete_method(
    node: Node,
    source: &[u8],
    concrete_methods: &mut HashMap<String, HashSet<String>>,
) {
    let Some(symbol) = super::common::current_go_symbol(node, source) else {
        return;
    };
    let Some((owner, method)) = symbol.rsplit_once('.') else {
        return;
    };
    concrete_methods
        .entry(owner.to_string())
        .or_default()
        .insert(method.to_string());
}
