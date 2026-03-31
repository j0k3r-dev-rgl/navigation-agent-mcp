use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use tree_sitter::{Node, Parser};

use super::language_analyzer::LanguageAnalyzer;
use super::types::{
    infer_public_language, normalize_public_endpoint_kind, normalize_public_symbol_kind,
    AnalyzerLanguage, CallerDefinition, CallerTarget, EndpointDefinition, FindCallersQuery,
    FindEndpointsQuery, FindSymbolQuery, SymbolDefinition,
};

pub struct TypeScriptAnalyzer;

impl LanguageAnalyzer for TypeScriptAnalyzer {
    fn language(&self) -> AnalyzerLanguage {
        AnalyzerLanguage::Typescript
    }

    fn supported_extensions(&self) -> &'static [&'static str] {
        &[".ts", ".tsx", ".js", ".jsx"]
    }

    fn find_symbols(
        &self,
        path: &Path,
        source: &str,
        _query: &FindSymbolQuery,
    ) -> Vec<SymbolDefinition> {
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

        let mut symbols = Vec::new();
        let public_language = infer_public_language(path);
        collect_symbols(
            tree.root_node(),
            source.as_bytes(),
            public_language.as_deref(),
            &mut symbols,
        );
        symbols
    }

    fn find_endpoints(
        &self,
        path: &Path,
        source: &str,
        _query: &FindEndpointsQuery,
    ) -> Vec<EndpointDefinition> {
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
        let route_path = derive_route_path_from_file(path);
        let is_route_file = route_path.is_some();

        let mut endpoints = Vec::new();
        collect_endpoints(
            tree.root_node(),
            source.as_bytes(),
            public_language.as_deref(),
            route_path.as_deref(),
            is_route_file,
            &mut endpoints,
        );
        endpoints
    }

    fn find_callers(
        &self,
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

    fn supports_framework(&self, framework: Option<&str>) -> bool {
        match framework {
            None => true,
            Some("react-router") => true,
            Some(_) => false,
        }
    }
}

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

fn parser_language_for_path(path: &Path) -> Option<tree_sitter_language::LanguageFn> {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
    {
        Some(extension) if extension == "ts" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT),
        Some(extension) if extension == "tsx" => Some(tree_sitter_typescript::LANGUAGE_TSX),
        Some(extension) if extension == "js" || extension == "jsx" => {
            Some(tree_sitter_javascript::LANGUAGE)
        }
        _ => None,
    }
}

fn collect_symbols(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
    symbols: &mut Vec<SymbolDefinition>,
) {
    if let Some(symbol) = extract_symbol(node, source, public_language) {
        symbols.push(symbol);
    }

    for index in 0..node.named_child_count() {
        if let Some(child) = node.named_child(index) {
            collect_symbols(child, source, public_language, symbols);
        }
    }
}

fn extract_symbol(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
) -> Option<SymbolDefinition> {
    let (name_node, raw_kind) = match node.kind() {
        "function_declaration" | "generator_function_declaration" => {
            (node.child_by_field_name("name")?, "function_declaration")
        }
        "class_declaration" | "abstract_class_declaration" => {
            (node.child_by_field_name("name")?, "class_declaration")
        }
        "interface_declaration" => (node.child_by_field_name("name")?, "interface_declaration"),
        "enum_declaration" => (node.child_by_field_name("name")?, "enum_declaration"),
        "type_alias_declaration" => (node.child_by_field_name("name")?, "type_alias_declaration"),
        "method_definition" | "method_signature" | "abstract_method_signature" => {
            let name_node = node.child_by_field_name("name")?;
            let symbol = node_text(name_node, source)?;
            let raw_kind = if symbol == "constructor" {
                "constructor"
            } else {
                "method_declaration"
            };

            return Some(SymbolDefinition {
                symbol,
                kind: normalize_public_symbol_kind(raw_kind),
                path: String::new(),
                line: (node.start_position().row + 1) as u32,
                language: public_language.map(str::to_string),
            });
        }
        "variable_declarator" => {
            let value = node.child_by_field_name("value")?;
            if !matches!(value.kind(), "arrow_function" | "function_expression") {
                return None;
            }
            (node.child_by_field_name("name")?, "function_declaration")
        }
        _ => return None,
    };

    Some(SymbolDefinition {
        symbol: node_text(name_node, source)?,
        kind: normalize_public_symbol_kind(raw_kind),
        path: String::new(),
        line: (node.start_position().row + 1) as u32,
        language: public_language.map(str::to_string),
    })
}

fn node_text(node: Node, source: &[u8]) -> Option<String> {
    node.utf8_text(source)
        .ok()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
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
    let import_path = if import_source.starts_with('.') {
        normalize_path(&base_dir.join(import_source))
    } else {
        normalize_path(&workspace_root.join(import_source))
    };
    let target = normalize_path(target_path);
    if import_path == target {
        return true;
    }

    let mut candidates = vec![import_path.clone()];
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

/// Derives the React Router 7 route path from a file path.
fn derive_route_path_from_file(path: &Path) -> Option<String> {
    let path_str = path.to_string_lossy();
    let routes_idx = path_str.find("/routes/")?;
    let route_file = &path_str[routes_idx + 8..];
    let route_name = route_file.rsplit_once('.')?.0;

    if route_name == "_index" || route_name.ends_with("/_index") {
        let parent = route_name.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
        if parent.is_empty() {
            return Some("/".to_string());
        }
        return Some(format!("/{}", parent.replace('.', "/")));
    }

    let segments: Vec<&str> = route_name
        .split('/')
        .last()
        .unwrap_or(route_name)
        .split('.')
        .collect();

    let path_segments: Vec<String> = segments
        .iter()
        .filter(|s| !s.starts_with('_'))
        .map(|s| {
            if s.starts_with('$') {
                format!(":{}", &s[1..])
            } else {
                s.to_string()
            }
        })
        .collect();

    if path_segments.is_empty() {
        return Some("/".to_string());
    }

    Some(format!("/{}", path_segments.join("/")))
}

fn collect_endpoints(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
    route_path: Option<&str>,
    is_route_file: bool,
    endpoints: &mut Vec<EndpointDefinition>,
) {
    if !is_route_file {
        return;
    }

    if node.kind() == "export_statement" {
        if let Some(endpoint) = extract_endpoint(node, source, public_language, route_path) {
            endpoints.push(endpoint);
        }
        return;
    }

    for index in 0..node.named_child_count() {
        if let Some(child) = node.named_child(index) {
            collect_endpoints(
                child,
                source,
                public_language,
                route_path,
                is_route_file,
                endpoints,
            );
        }
    }
}

fn extract_endpoint(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
    route_path: Option<&str>,
) -> Option<EndpointDefinition> {
    let (name, raw_kind) = match node.kind() {
        "function_declaration" | "generator_function_declaration" => {
            let name_node = node.child_by_field_name("name")?;
            let name = node_text(name_node, source)?;
            if name != "loader" && name != "action" {
                return None;
            }
            let kind = name.clone();
            (name, kind)
        }
        "variable_declarator" => {
            let name_node = node.child_by_field_name("name")?;
            let name = node_text(name_node, source)?;
            if name != "loader" && name != "action" {
                return None;
            }
            let value = node.child_by_field_name("value")?;
            if !matches!(value.kind(), "arrow_function" | "function_expression") {
                return None;
            }
            let kind = name.clone();
            (name, kind)
        }
        "export_statement" => {
            let declaration = node.child_by_field_name("declaration")?;
            match declaration.kind() {
                "function_declaration"
                | "generator_function_declaration"
                | "variable_declarator" => {
                    return extract_endpoint(declaration, source, public_language, route_path);
                }
                "lexical_declaration" => {
                    for i in 0..declaration.named_child_count() {
                        if let Some(child) = declaration.named_child(i) {
                            if child.kind() == "variable_declarator" {
                                return extract_endpoint(
                                    child,
                                    source,
                                    public_language,
                                    route_path,
                                );
                            }
                        }
                    }
                    return None;
                }
                _ => return None,
            }
        }
        _ => return None,
    };

    Some(EndpointDefinition {
        name,
        kind: normalize_public_endpoint_kind(&raw_kind),
        path: route_path.map(str::to_string),
        file: String::new(),
        line: (node.start_position().row + 1) as u32,
        language: public_language.map(str::to_string),
        framework: Some("react-router".to_string()),
    })
}
