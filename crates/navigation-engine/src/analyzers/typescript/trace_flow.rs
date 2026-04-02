use std::collections::HashMap;
use std::path::Path;

use tree_sitter::{Node, Parser};

use super::super::types::{infer_public_language, CalleeDefinition, FindCalleesQuery};
use super::common::{find_workspace_root, node_text, parser_language_for_path};

struct TypeScriptFileContext {
    imports: HashMap<String, ResolvedImport>,
    local_definitions: std::collections::HashSet<String>,
}

#[derive(Debug, Clone)]
enum ResolvedImport {
    Local { path: String },
    External { package: String },
    Unknown,
}

impl TypeScriptFileContext {
    fn new(
        _path_mappings: Vec<(String, Vec<String>)>,
        _base_url: Option<String>,
        local_definitions: std::collections::HashSet<String>,
    ) -> Self {
        Self {
            imports: HashMap::new(),
            local_definitions,
        }
    }

    fn is_callee_from_project(&self, receiver: Option<&str>, callee_name: &str) -> bool {
        if let Some(receiver_name) = receiver {
            if let Some(resolved) = self.imports.get(receiver_name) {
                return matches!(resolved, ResolvedImport::Local { .. });
            }

            if self.local_definitions.contains(receiver_name) {
                return true;
            }

            return false;
        }

        if let Some(resolved) = self.imports.get(callee_name) {
            return matches!(resolved, ResolvedImport::Local { .. });
        }

        if self.local_definitions.contains(callee_name) {
            return true;
        }

        false
    }
}

struct CalleeContext<'a> {
    target_symbol: &'a str,
    current_file: &'a Path,
    public_language: Option<&'a str>,
    file_context: Option<TypeScriptFileContext>,
}

#[derive(Clone)]
struct FunctionContextForCallee {
    _depth: usize,
}

pub(super) fn find_callees(
    path: &Path,
    source: &str,
    query: &FindCalleesQuery,
) -> Vec<CalleeDefinition> {
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

    let workspace_root = find_workspace_root(path);
    let (path_mappings, base_url) = find_tsconfig(&workspace_root).unwrap_or_default();

    let imports = extract_typescript_imports(
        tree.root_node(),
        source.as_bytes(),
        path,
        &workspace_root,
        &path_mappings,
        base_url.as_deref(),
    );

    let local_definitions = extract_local_definitions(tree.root_node(), source.as_bytes());

    let file_context = TypeScriptFileContext::new(path_mappings, base_url, local_definitions);
    let file_context = TypeScriptFileContext {
        imports,
        ..file_context
    };

    let mut callees = Vec::new();
    let context = CalleeContext {
        target_symbol: &query.target_symbol,
        current_file: path,
        public_language: public_language.as_deref(),
        file_context: Some(file_context),
    };

    collect_callees(
        tree.root_node(),
        source.as_bytes(),
        None,
        &context,
        &mut callees,
    );

    for callee in &mut callees {
        if !callee.path.starts_with('/') {
            callee.path = path.to_string_lossy().replace('\\', "/");
        }
    }

    callees
}

fn find_tsconfig(workspace_root: &Path) -> Option<(Vec<(String, Vec<String>)>, Option<String>)> {
    let tsconfig_path = workspace_root.join("tsconfig.json");
    if !tsconfig_path.exists() {
        return None;
    }

    let content = std::fs::read_to_string(&tsconfig_path).ok()?;
    parse_tsconfig_paths(&content)
}

fn parse_tsconfig_paths(content: &str) -> Option<(Vec<(String, Vec<String>)>, Option<String>)> {
    let json: serde_json::Value = serde_json::from_str(content).ok()?;
    let compiler_options = json.get("compilerOptions")?;

    let base_url = compiler_options
        .get("baseUrl")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let mut path_mappings = Vec::new();
    if let Some(paths) = compiler_options.get("paths") {
        if let Some(obj) = paths.as_object() {
            for (key, value) in obj {
                let targets: Vec<String> = value
                    .as_array()?
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                if !targets.is_empty() {
                    path_mappings.push((key.clone(), targets));
                }
            }
        }
    }

    Some((path_mappings, base_url))
}

fn resolve_import_path(
    import_path: &str,
    current_file: &Path,
    workspace_root: &Path,
    path_mappings: &[(String, Vec<String>)],
    _base_url: Option<&str>,
) -> Option<String> {
    if !import_path.starts_with('.') && !import_path.starts_with('/') {
        for (pattern, targets) in path_mappings {
            if let Some(resolved) =
                resolve_path_alias(import_path, pattern, targets, workspace_root)
            {
                return Some(resolved);
            }
        }

        if !import_path.starts_with("~") && !import_path.starts_with("@/") {
            return Some(format!("<node_modules>/{}", import_path));
        }

        return None;
    }

    let current_dir = current_file.parent()?;
    let resolved = current_dir.join(import_path);

    if let Ok(canonical) = resolved.canonicalize() {
        return Some(canonical.to_string_lossy().to_string());
    }

    let path_str = resolved.to_string_lossy();
    for ext in &[".ts", ".tsx", ".js", ".jsx"] {
        let path_with_ext = std::path::PathBuf::from(format!("{}{}", path_str, ext));
        if let Ok(canonical) = path_with_ext.canonicalize() {
            return Some(canonical.to_string_lossy().to_string());
        }
    }

    for ext in &[".ts", ".tsx", ".js", ".jsx"] {
        let index_path = resolved.join(format!("index{}", ext));
        if let Ok(canonical) = index_path.canonicalize() {
            return Some(canonical.to_string_lossy().to_string());
        }
    }

    None
}

fn resolve_path_alias(
    import_path: &str,
    pattern: &str,
    targets: &[String],
    workspace_root: &Path,
) -> Option<String> {
    if pattern.ends_with("/*") {
        let prefix = &pattern[..pattern.len() - 2];
        if import_path.starts_with(prefix) && import_path.len() > prefix.len() {
            let suffix = &import_path[prefix.len()..];
            let suffix = suffix.strip_prefix('/').unwrap_or(suffix);

            let target = targets.first()?;
            if target.ends_with("/*") {
                let target_prefix = &target[..target.len() - 2];
                let target_prefix = target_prefix.strip_prefix("./").unwrap_or(target_prefix);
                let resolved_path = workspace_root.join(target_prefix).join(suffix);

                if let Ok(canonical) = resolved_path.canonicalize() {
                    return Some(canonical.to_string_lossy().to_string());
                }

                let path_str = resolved_path.to_string_lossy();
                for ext in &[".ts", ".tsx", ".js", ".jsx"] {
                    let path_with_ext = format!("{}{}", path_str, ext);
                    let path_with_ext = std::path::PathBuf::from(path_with_ext);
                    if let Ok(canonical) = path_with_ext.canonicalize() {
                        return Some(canonical.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    None
}

fn extract_typescript_imports(
    root: Node,
    source: &[u8],
    current_file: &Path,
    workspace_root: &Path,
    path_mappings: &[(String, Vec<String>)],
    base_url: Option<&str>,
) -> HashMap<String, ResolvedImport> {
    let mut imports = HashMap::new();

    for index in 0..root.named_child_count() {
        if let Some(child) = root.named_child(index) {
            match child.kind() {
                "import_statement" | "import_declaration" => {
                    extract_import_declaration(
                        child,
                        source,
                        current_file,
                        workspace_root,
                        path_mappings,
                        base_url,
                        &mut imports,
                    );
                }
                "expression_statement" => {
                    extract_require_call(
                        child,
                        source,
                        current_file,
                        workspace_root,
                        path_mappings,
                        base_url,
                        &mut imports,
                    );
                }
                _ => {}
            }
        }
    }

    imports
}

fn extract_local_definitions(root: Node, source: &[u8]) -> std::collections::HashSet<String> {
    let mut definitions = std::collections::HashSet::new();

    for index in 0..root.named_child_count() {
        if let Some(child) = root.named_child(index) {
            extract_definitions_from_node(child, source, &mut definitions);
        }
    }

    definitions
}

fn extract_definitions_from_node(
    node: Node,
    source: &[u8],
    definitions: &mut std::collections::HashSet<String>,
) {
    match node.kind() {
        "function_declaration" | "generator_function_declaration" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Some(name) = node_text(name_node, source) {
                    definitions.insert(name);
                }
            }
            return;
        }
        "class_declaration" | "class" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Some(name) = node_text(name_node, source) {
                    definitions.insert(name);
                }
            }
            return;
        }
        "variable_declaration" | "lexical_declaration" => {
            extract_variable_names(node, source, definitions);
            return;
        }
        "export_statement" | "export_declaration" => {
            for index in 0..node.named_child_count() {
                if let Some(child) = node.named_child(index) {
                    match child.kind() {
                        "function_declaration"
                        | "class_declaration"
                        | "variable_declaration"
                        | "lexical_declaration" => {
                            extract_definitions_from_node(child, source, definitions);
                        }
                        _ => {}
                    }
                }
            }
            return;
        }
        "type_alias_declaration" | "interface_declaration" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Some(name) = node_text(name_node, source) {
                    definitions.insert(name);
                }
            }
            return;
        }
        _ => {}
    }

    for index in 0..node.named_child_count() {
        if let Some(child) = node.named_child(index) {
            extract_definitions_from_node(child, source, definitions);
        }
    }
}

fn extract_variable_names(
    node: Node,
    source: &[u8],
    definitions: &mut std::collections::HashSet<String>,
) {
    for index in 0..node.named_child_count() {
        if let Some(child) = node.named_child(index) {
            if child.kind() == "variable_declarator" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    extract_names_from_pattern(name_node, source, definitions);
                }
            }
        }
    }
}

fn extract_names_from_pattern(
    node: Node,
    source: &[u8],
    definitions: &mut std::collections::HashSet<String>,
) {
    match node.kind() {
        "identifier" => {
            if let Some(name) = node_text(node, source) {
                definitions.insert(name);
            }
        }
        "object_pattern" => {
            for index in 0..node.named_child_count() {
                if let Some(child) = node.named_child(index) {
                    if child.kind() == "shorthand_property_identifier_pattern"
                        || child.kind() == "identifier"
                    {
                        if let Some(name) = node_text(child, source) {
                            definitions.insert(name);
                        }
                    } else if child.kind() == "pair_pattern" {
                        if let Some(value) = child.child_by_field_name("value") {
                            extract_names_from_pattern(value, source, definitions);
                        }
                    }
                }
            }
        }
        "array_pattern" => {
            for index in 0..node.named_child_count() {
                if let Some(child) = node.named_child(index) {
                    extract_names_from_pattern(child, source, definitions);
                }
            }
        }
        _ => {}
    }
}

fn extract_import_declaration(
    node: Node,
    source: &[u8],
    current_file: &Path,
    workspace_root: &Path,
    path_mappings: &[(String, Vec<String>)],
    base_url: Option<&str>,
    imports: &mut HashMap<String, ResolvedImport>,
) {
    let mut import_clause = None;
    let mut source_path = None;

    for index in 0..node.named_child_count() {
        if let Some(child) = node.named_child(index) {
            match child.kind() {
                "import_clause" => {
                    import_clause = Some(child);
                }
                "string" | "string_fragment" => {
                    source_path = node_text(child, source);
                }
                _ => {}
            }
        }
    }

    let Some(source_path) = source_path else {
        return;
    };

    let source_path = source_path
        .trim_start_matches('"')
        .trim_end_matches('"')
        .trim_start_matches('\'')
        .trim_end_matches('\'')
        .to_string();

    let resolved = resolve_import_path(
        &source_path,
        current_file,
        workspace_root,
        path_mappings,
        base_url,
    );

    let import_kind = if source_path.starts_with(".") {
        if let Some(resolved_path) = resolved {
            ResolvedImport::Local {
                path: resolved_path,
            }
        } else {
            ResolvedImport::Unknown
        }
    } else if source_path.starts_with("~") || source_path.starts_with("@/") {
        if let Some(resolved_path) = resolved {
            ResolvedImport::Local {
                path: resolved_path,
            }
        } else {
            ResolvedImport::Unknown
        }
    } else {
        ResolvedImport::External {
            package: source_path
                .split('/')
                .next()
                .unwrap_or(&source_path)
                .to_string(),
        }
    };

    if let Some(clause) = import_clause {
        extract_import_specifiers(clause, source, &import_kind, imports);
    }
}

fn extract_import_specifiers(
    clause: Node,
    source: &[u8],
    import_kind: &ResolvedImport,
    imports: &mut HashMap<String, ResolvedImport>,
) {
    for index in 0..clause.named_child_count() {
        if let Some(child) = clause.named_child(index) {
            match child.kind() {
                "named_imports" => {
                    for i in 0..child.named_child_count() {
                        if let Some(spec) = child.named_child(i) {
                            if spec.kind() == "import_specifier" {
                                extract_single_import_specifier(spec, source, import_kind, imports);
                            }
                        }
                    }
                }
                "import_specifier" => {
                    extract_single_import_specifier(child, source, import_kind, imports);
                }
                "identifier" => {
                    if let Some(name) = node_text(child, source) {
                        imports.insert(name, import_kind.clone());
                    }
                }
                "namespace_import" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        if let Some(name) = node_text(name_node, source) {
                            imports.insert(name, import_kind.clone());
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn extract_single_import_specifier(
    spec: Node,
    source: &[u8],
    import_kind: &ResolvedImport,
    imports: &mut HashMap<String, ResolvedImport>,
) {
    let mut local_name = None;

    for i in 0..spec.named_child_count() {
        if let Some(spec_child) = spec.named_child(i) {
            if spec_child.kind() == "identifier" && local_name.is_none() {
                local_name = node_text(spec_child, source);
            }
        }
    }

    let name = local_name.unwrap_or_default();
    if !name.is_empty() {
        imports.insert(name, import_kind.clone());
    }
}

fn extract_require_call(
    node: Node,
    source: &[u8],
    current_file: &Path,
    workspace_root: &Path,
    path_mappings: &[(String, Vec<String>)],
    base_url: Option<&str>,
    imports: &mut HashMap<String, ResolvedImport>,
) {
    let text = node_text(node, source).unwrap_or_default();

    if text.contains("require(") {
        if let Some(require_idx) = text.find("require(") {
            let after_require = &text[require_idx + 8..];
            if after_require
                .find('"')
                .or_else(|| after_require.find('\''))
                .is_some()
            {
                let path_start = after_require
                    .chars()
                    .position(|c| c == '"' || c == '\'')
                    .unwrap_or(0)
                    + 1;
                if let Some(path_end) = after_require[path_start..]
                    .chars()
                    .position(|c| c == '"' || c == '\'')
                {
                    let import_path = &after_require[path_start..path_start + path_end];

                    if let Some(eq_idx) = text[..require_idx].find('=') {
                        let before_eq = &text[..eq_idx];
                        if let Some(const_idx) = before_eq
                            .rfind("const ")
                            .or_else(|| before_eq.rfind("let "))
                            .or_else(|| before_eq.rfind("var "))
                        {
                            let var_name = before_eq[const_idx + 6..]
                                .trim()
                                .split_whitespace()
                                .next()
                                .unwrap_or("");
                            if !var_name.is_empty() && !var_name.contains('{') {
                                let resolved = resolve_import_path(
                                    import_path,
                                    current_file,
                                    workspace_root,
                                    path_mappings,
                                    base_url,
                                );

                                let import_kind = if import_path.starts_with(".")
                                    || import_path.starts_with("~")
                                    || import_path.starts_with("@/")
                                {
                                    if let Some(path) = resolved {
                                        ResolvedImport::Local { path }
                                    } else {
                                        ResolvedImport::Unknown
                                    }
                                } else {
                                    ResolvedImport::External {
                                        package: import_path
                                            .split('/')
                                            .next()
                                            .unwrap_or(import_path)
                                            .to_string(),
                                    }
                                };

                                imports.insert(var_name.to_string(), import_kind);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn collect_callees(
    node: Node,
    source: &[u8],
    current_function: Option<FunctionContextForCallee>,
    ctx: &CalleeContext,
    callees: &mut Vec<CalleeDefinition>,
) {
    let is_target_function = match node.kind() {
        "function_declaration" | "generator_function_declaration" => node
            .child_by_field_name("name")
            .and_then(|n| node_text(n, source))
            .map(|name| name == ctx.target_symbol)
            .unwrap_or(false),
        "method_definition" => node
            .child_by_field_name("name")
            .and_then(|n| node_text(n, source))
            .map(|name| name == ctx.target_symbol)
            .unwrap_or(false),
        "variable_declarator" => {
            let value = node.child_by_field_name("value").map(|v| v.kind());
            if value == Some("arrow_function") || value == Some("function_expression") {
                node.child_by_field_name("name")
                    .and_then(|n| node_text(n, source))
                    .map(|name| name == ctx.target_symbol)
                    .unwrap_or(false)
            } else {
                false
            }
        }
        _ => false,
    };

    let next_function = if is_target_function {
        Some(FunctionContextForCallee { _depth: 0 })
    } else {
        current_function.clone()
    };

    if is_target_function || current_function.is_some() {
        if node.kind() == "call_expression" {
            if let Some(callee) = extract_callee(node, source, ctx) {
                callees.push(callee);
            }
        }
    }

    for index in 0..node.named_child_count() {
        if let Some(child) = node.named_child(index) {
            collect_callees(child, source, next_function.clone(), ctx, callees);
        }
    }
}

fn extract_callee(node: Node, source: &[u8], ctx: &CalleeContext) -> Option<CalleeDefinition> {
    if node.kind() != "call_expression" {
        return None;
    }

    let callee = node.child_by_field_name("function")?;

    let (callee_name, receiver_type, _root_object) = match callee.kind() {
        "identifier" => {
            let name = node_text(callee, source)?;
            (name, None, None)
        }
        "member_expression" => {
            let object = callee
                .child_by_field_name("object")
                .and_then(|item| node_text(item, source));
            let property = callee
                .child_by_field_name("property")
                .and_then(|item| node_text(item, source));
            let name = property.unwrap_or_else(|| node_text(callee, source).unwrap_or_default());
            let root = find_root_object(callee, source);
            (name, object, root)
        }
        _ => return None,
    };

    if let Some(ref file_ctx) = ctx.file_context {
        if !file_ctx.is_callee_from_project(receiver_type.as_deref(), &callee_name) {
            return None;
        }
    } else {
        return None;
    }

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
        language: ctx.public_language.map(str::to_string),
        snippet: node_text(node, source),
    })
}

fn find_root_object(member_expr: Node, source: &[u8]) -> Option<String> {
    let mut current = member_expr;

    while current.kind() == "member_expression" {
        if let Some(object) = current.child_by_field_name("object") {
            if object.kind() != "member_expression" {
                return node_text(object, source);
            }
            current = object;
        } else {
            break;
        }
    }

    None
}
