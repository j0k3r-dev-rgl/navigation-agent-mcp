use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use tree_sitter::{Node, Parser};

use super::super::types::{
    infer_public_language, CallerDefinition, CallerTarget, FindCallersQuery,
};
use super::common::{derive_route_path_from_file, node_text, parser_language_for_path};

struct ImportAliases {
    direct: BTreeSet<String>,
    namespace: BTreeSet<String>,
}

struct CallerContext<'a> {
    target_symbol: &'a str,
    target_path: &'a Path,
    current_file: &'a Path,
    public_language: Option<&'a str>,
    route_path: Option<String>,
    aliases: ImportAliases,
    same_file_target: bool,
}

#[derive(Clone)]
struct FunctionContext {
    caller: String,
    caller_symbol: Option<String>,
    probable_entry_point_reasons: Vec<String>,
}

pub(super) fn find_callers(
    workspace_root: &Path,
    path: &Path,
    source: &str,
    query: &FindCallersQuery,
) -> Vec<CallerDefinition> {
    let Some(language) = parser_language_for_path(path) else {
        return Vec::new();
    };

    let mut parser = Parser::new();
    if parser.set_language(&language.into()).is_err() {
        return Vec::new();
    }

    let Some(tree) = parser.parse(source, None) else {
        return Vec::new();
    };

    let public_language = infer_public_language(path);
    let aliases = collect_import_aliases(
        workspace_root,
        path,
        tree.root_node(),
        source.as_bytes(),
        query,
    );
    let same_file_target = normalize_path(path) == normalize_path(&query.target_path);

    if !same_file_target && aliases.direct.is_empty() && aliases.namespace.is_empty() {
        return Vec::new();
    }

    let context = CallerContext {
        target_symbol: query.target_symbol.as_str(),
        target_path: query.target_path.as_path(),
        current_file: path,
        public_language: public_language.as_deref(),
        route_path: derive_route_path_from_file(path),
        aliases,
        same_file_target,
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

fn collect_import_aliases(
    workspace_root: &Path,
    current_file: &Path,
    root: Node,
    source: &[u8],
    query: &FindCallersQuery,
) -> ImportAliases {
    let mut aliases = ImportAliases {
        direct: BTreeSet::new(),
        namespace: BTreeSet::new(),
    };

    for i in 0..root.named_child_count() {
        let Some(child) = root.named_child(i) else {
            continue;
        };
        if child.kind() != "import_statement" {
            continue;
        }

        let Some(source_node) = child.child_by_field_name("source") else {
            continue;
        };
        let Some(import_source) = node_text(source_node, source) else {
            continue;
        };
        let normalized_source = import_source.trim_matches(&['\'', '"'][..]);
        if !import_matches_target(
            workspace_root,
            current_file,
            normalized_source,
            &query.target_path,
        ) {
            continue;
        }

        for j in 0..child.named_child_count() {
            let Some(named) = child.named_child(j) else {
                continue;
            };
            match named.kind() {
                "import_clause" => collect_from_import_clause(named, source, query, &mut aliases),
                "identifier" => {
                    aliases.direct.insert(query.target_symbol.to_string());
                    aliases
                        .direct
                        .insert(node_text(named, source).unwrap_or_default());
                }
                _ => {}
            }
        }
    }

    aliases
}

fn collect_from_import_clause(
    node: Node,
    source: &[u8],
    query: &FindCallersQuery,
    aliases: &mut ImportAliases,
) {
    for i in 0..node.named_child_count() {
        let Some(child) = node.named_child(i) else {
            continue;
        };
        match child.kind() {
            "identifier" => {
                aliases.direct.insert(
                    child
                        .utf8_text(source)
                        .ok()
                        .unwrap_or_default()
                        .trim()
                        .to_string(),
                );
            }
            "named_imports" => {
                for j in 0..child.named_child_count() {
                    let Some(specifier) = child.named_child(j) else {
                        continue;
                    };
                    if specifier.kind() != "import_specifier" {
                        continue;
                    }
                    let imported = specifier
                        .child_by_field_name("name")
                        .and_then(|item| node_text(item, source));
                    let alias = specifier
                        .child_by_field_name("alias")
                        .and_then(|item| node_text(item, source))
                        .or_else(|| imported.clone());
                    if imported.as_deref() == Some(query.target_symbol.as_str()) {
                        if let Some(alias) = alias {
                            aliases.direct.insert(alias);
                        }
                    }
                }
            }
            "namespace_import" => {
                if let Some(name) = child
                    .child_by_field_name("name")
                    .and_then(|item| node_text(item, source))
                {
                    aliases.namespace.insert(name);
                }
            }
            _ => {}
        }
    }
}

fn import_matches_target(
    workspace_root: &Path,
    current_file: &Path,
    import_source: &str,
    target_path: &Path,
) -> bool {
    let Some(base_dir) = current_file.parent() else {
        return false;
    };

    let import_source = import_source.trim().trim_matches(&['\'', '"'][..]);

    if !import_source.starts_with('.') && !import_source.starts_with("~/") {
        return false;
    }

    if import_source.starts_with("~/") {
        let remainder = &import_source[2..];

        let has_extension = remainder.ends_with(".ts")
            || remainder.ends_with(".tsx")
            || remainder.ends_with(".js")
            || remainder.ends_with(".jsx");

        let candidates_dirs = ["app", "src"];
        for candidate_dir in candidates_dirs {
            let candidate_path = workspace_root.join(candidate_dir).join(remainder);

            if has_extension {
                if candidate_path.exists()
                    && normalize_path(&candidate_path) == normalize_path(target_path)
                {
                    return true;
                }
            } else if candidate_path.exists()
                || candidate_path.with_extension("ts").exists()
                || candidate_path.with_extension("tsx").exists()
                || candidate_path.join("index.ts").exists()
                || candidate_path.join("index.tsx").exists()
            {
                return import_matches_candidates(&normalize_path(&candidate_path), target_path);
            }
        }

        let fallback_path = workspace_root.join(remainder);
        return import_matches_candidates(&normalize_path(&fallback_path), target_path);
    }

    let import_path = if import_source.starts_with('.') {
        normalize_path(&base_dir.join(import_source))
    } else {
        normalize_path(&workspace_root.join(import_source))
    };
    let target = normalize_path(target_path);
    if import_path == target {
        return true;
    }

    import_matches_candidates(&import_path, &target)
}

fn import_matches_candidates(import_path: &Path, target_path: &Path) -> bool {
    let target = normalize_path(target_path);
    if normalize_path(import_path) == target {
        return true;
    }

    let mut candidates = vec![import_path.to_path_buf()];
    for extension in ["ts", "tsx", "js", "jsx"] {
        candidates.push(PathBuf::from(format!(
            "{}.{}",
            import_path.to_string_lossy(),
            extension
        )));
        candidates.push(import_path.join(format!("index.{}", extension)));
    }

    candidates
        .into_iter()
        .any(|candidate| normalize_path(&candidate) == target)
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::CurDir => {}
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn walk_for_callers(
    node: Node,
    source: &[u8],
    current_context: Option<FunctionContext>,
    ctx: &CallerContext,
    callers: &mut Vec<CallerDefinition>,
) {
    let next_context = derive_function_context(node, source, current_context.clone(), ctx)
        .or(current_context.clone());

    if let Some(function_context) = &next_context {
        if let Some(caller) = extract_call_reference(node, source, function_context, ctx) {
            callers.push(caller);
        }
    }

    for index in 0..node.named_child_count() {
        if let Some(child) = node.named_child(index) {
            walk_for_callers(child, source, next_context.clone(), ctx, callers);
        }
    }
}

fn derive_function_context(
    node: Node,
    source: &[u8],
    inherited: Option<FunctionContext>,
    ctx: &CallerContext,
) -> Option<FunctionContext> {
    match node.kind() {
        "function_declaration" | "generator_function_declaration" => {
            let name = node
                .child_by_field_name("name")
                .and_then(|item| node_text(item, source))?;
            Some(build_function_context(name, node, ctx))
        }
        "method_definition" => {
            let name = node
                .child_by_field_name("name")
                .and_then(|item| node_text(item, source))?;
            Some(build_function_context(name, node, ctx))
        }
        "variable_declarator" => {
            let value = node.child_by_field_name("value")?;
            if !matches!(value.kind(), "arrow_function" | "function_expression") {
                return inherited;
            }
            let name = node
                .child_by_field_name("name")
                .and_then(|item| node_text(item, source))?;
            Some(build_function_context(name, node, ctx))
        }
        _ => inherited,
    }
}

fn build_function_context(name: String, node: Node, ctx: &CallerContext) -> FunctionContext {
    let mut reasons = Vec::new();
    if ctx.route_path.is_some() && is_exported(node) && matches!(name.as_str(), "loader" | "action")
    {
        reasons.push("route module export".to_string());
    }

    FunctionContext {
        caller: name.clone(),
        caller_symbol: Some(name),
        probable_entry_point_reasons: reasons,
    }
}

fn is_exported(node: Node) -> bool {
    let mut current = node;
    while let Some(parent) = current.parent() {
        if parent.kind() == "export_statement" {
            return true;
        }
        current = parent;
    }
    false
}

fn extract_call_reference(
    node: Node,
    source: &[u8],
    function_context: &FunctionContext,
    ctx: &CallerContext,
) -> Option<CallerDefinition> {
    if node.kind() != "call_expression" {
        return None;
    }
    if function_context.caller_symbol.as_deref() == Some(ctx.target_symbol) && ctx.same_file_target
    {
        return None;
    }

    let callee = node.child_by_field_name("function")?;
    let matched = match callee.kind() {
        "identifier" => {
            let identifier = node_text(callee, source)?;
            (ctx.same_file_target && identifier == ctx.target_symbol)
                || ctx.aliases.direct.contains(&identifier)
        }
        "member_expression" => {
            let object = callee
                .child_by_field_name("object")
                .and_then(|item| node_text(item, source));
            let property = callee
                .child_by_field_name("property")
                .and_then(|item| node_text(item, source));
            property.as_deref() == Some(ctx.target_symbol)
                && object
                    .as_deref()
                    .is_some_and(|value| ctx.aliases.namespace.contains(value))
        }
        _ => false,
    };

    if !matched {
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
        receiver_type: None,
        calls: CallerTarget {
            path: ctx.target_path.to_string_lossy().replace('\\', "/"),
            symbol: ctx.target_symbol.to_string(),
        },
        probable_entry_point_reasons: function_context.probable_entry_point_reasons.clone(),
    })
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
