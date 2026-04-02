use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tree_sitter::Node;

use super::super::language_analyzer::LanguageAnalyzer;
use super::super::types::{
    AnalyzerLanguage, CalleeDefinition, CallerDefinition, EndpointDefinition, FindCalleesQuery,
    FindCallersQuery, FindEndpointsQuery, FindSymbolQuery, SymbolDefinition,
};

pub struct GoAnalyzer;

impl LanguageAnalyzer for GoAnalyzer {
    fn language(&self) -> AnalyzerLanguage {
        AnalyzerLanguage::Go
    }

    fn supported_extensions(&self) -> &'static [&'static str] {
        &[".go"]
    }

    fn find_symbols(
        &self,
        path: &Path,
        source: &str,
        query: &FindSymbolQuery,
    ) -> Vec<SymbolDefinition> {
        super::find_symbol::find_symbols(path, source, query)
    }

    fn find_endpoints(
        &self,
        _path: &Path,
        _source: &str,
        _query: &FindEndpointsQuery,
    ) -> Vec<EndpointDefinition> {
        Vec::new()
    }

    fn find_callees(
        &self,
        path: &Path,
        source: &str,
        query: &FindCalleesQuery,
    ) -> Vec<CalleeDefinition> {
        super::trace_flow::find_callees(path, source, query)
    }

    fn find_callers(
        &self,
        workspace_root: &Path,
        path: &Path,
        source: &str,
        query: &FindCallersQuery,
    ) -> Vec<CallerDefinition> {
        super::trace_callers::find_callers(workspace_root, path, source, query)
    }
}

#[derive(Default, Clone)]
pub(super) struct GoFileContext {
    pub imports: HashMap<String, PathBuf>,
    pub struct_fields: HashMap<String, HashMap<String, String>>,
}

#[derive(Clone)]
pub(super) struct GoFunctionContext {
    pub symbol: String,
    pub local_bindings: HashMap<String, String>,
}

#[derive(Clone)]
pub(super) struct ResolvedGoCall {
    pub symbol: String,
    pub destination: PathBuf,
    pub receiver_type: Option<String>,
}

#[derive(Clone)]
pub(super) enum GoTargetSymbol {
    Function(String),
    Method { owner: String, name: String },
}

pub(super) fn node_text(node: Node, source: &[u8]) -> Option<String> {
    node.utf8_text(source)
        .ok()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub(super) fn simplify_go_type_name(value: &str) -> String {
    value
        .trim()
        .trim_start_matches('*')
        .rsplit('.')
        .next()
        .unwrap_or(value)
        .to_string()
}

pub(super) fn go_module_root(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(dir) = current {
        if dir.join("go.mod").exists() {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    None
}

pub(super) fn go_module_name(workspace_root: &Path) -> Option<String> {
    let go_mod = std::fs::read_to_string(workspace_root.join("go.mod")).ok()?;
    for line in go_mod.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("module ") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

pub(super) fn import_path_to_dir(
    workspace_root: &Path,
    module_name: Option<&str>,
    import_path: &str,
) -> Option<PathBuf> {
    let module_name = module_name?;
    let normalized = import_path.trim().trim_matches('"');
    let relative = normalized.strip_prefix(module_name)?;
    let relative = relative.trim_start_matches('/');
    Some(workspace_root.join(relative))
}

pub(super) fn normalize_go_target_symbol(value: &str) -> GoTargetSymbol {
    if let Some((owner, name)) = value.rsplit_once('.') {
        GoTargetSymbol::Method {
            owner: owner.to_string(),
            name: name.to_string(),
        }
    } else {
        GoTargetSymbol::Function(value.to_string())
    }
}

impl GoTargetSymbol {
    pub fn display(&self) -> String {
        match self {
            Self::Function(name) => name.clone(),
            Self::Method { owner, name } => format!("{owner}.{name}"),
        }
    }

    pub fn matches(
        &self,
        candidate_symbol: &str,
        candidate_path: &Path,
        target_path: &Path,
    ) -> bool {
        let symbol_matches = match self {
            Self::Function(name) => candidate_symbol == name,
            Self::Method { owner, name } => candidate_symbol == format!("{owner}.{name}"),
        };

        symbol_matches && go_path_matches_target(candidate_path, target_path)
    }
}

pub(super) fn go_path_matches_target(candidate_path: &Path, target_path: &Path) -> bool {
    let candidate = candidate_path.to_string_lossy().replace('\\', "/");
    let target = target_path.to_string_lossy().replace('\\', "/");

    candidate == target || target.starts_with(&(candidate + "/"))
}

pub(super) fn current_go_symbol(node: Node, source: &[u8]) -> Option<String> {
    match node.kind() {
        "function_declaration" => node
            .child_by_field_name("name")
            .or_else(|| {
                (0..node.named_child_count())
                    .filter_map(|index| node.named_child(index))
                    .find(|child| child.kind() == "identifier")
            })
            .and_then(|n| node_text(n, source)),
        "method_declaration" => {
            let name = node
                .child_by_field_name("name")
                .or_else(|| {
                    (0..node.named_child_count())
                        .filter_map(|index| node.named_child(index))
                        .find(|child| child.kind() == "field_identifier")
                })
                .and_then(|n| node_text(n, source))?;
            let receiver = node.child_by_field_name("receiver").or_else(|| {
                (0..node.named_child_count())
                    .filter_map(|index| node.named_child(index))
                    .find(|child| child.kind() == "parameter_list")
            })?;
            let owner = extract_receiver_type(receiver, source)?;
            Some(format!("{owner}.{name}"))
        }
        _ => None,
    }
}

pub(super) fn extract_go_function_context(node: Node, source: &[u8]) -> Option<GoFunctionContext> {
    let symbol = current_go_symbol(node, source)?;
    Some(GoFunctionContext {
        symbol,
        local_bindings: collect_local_bindings(node, source),
    })
}

pub(super) fn extract_go_file_context(
    root: Node,
    source: &[u8],
    workspace_root: Option<&Path>,
    module_name: Option<&str>,
) -> GoFileContext {
    let mut context = GoFileContext::default();

    for index in 0..root.named_child_count() {
        let Some(child) = root.named_child(index) else {
            continue;
        };
        match child.kind() {
            "import_declaration" => collect_imports(
                child,
                source,
                workspace_root,
                module_name,
                &mut context.imports,
            ),
            "type_declaration" => collect_struct_fields(child, source, &mut context.struct_fields),
            _ => {}
        }
    }

    context
}

pub(super) fn resolve_go_call_target(
    function: Node,
    source: &[u8],
    file_ctx: &GoFileContext,
    current_file: &Path,
    function_context: &GoFunctionContext,
) -> Option<ResolvedGoCall> {
    match function.kind() {
        "identifier" => Some(ResolvedGoCall {
            symbol: node_text(function, source)?,
            destination: current_file.to_path_buf(),
            receiver_type: None,
        }),
        "selector_expression" => {
            resolve_selector_expression(function, source, file_ctx, current_file, function_context)
        }
        _ => None,
    }
}

fn resolve_selector_expression(
    node: Node,
    source: &[u8],
    file_ctx: &GoFileContext,
    current_file: &Path,
    function_context: &GoFunctionContext,
) -> Option<ResolvedGoCall> {
    let operand = node
        .child_by_field_name("operand")
        .or_else(|| node.named_child(0))?;
    let field = node
        .child_by_field_name("field")
        .or_else(|| node.named_child(1))?;
    let field_name = node_text(field, source)?;

    if operand.kind() == "identifier" {
        let operand_name = node_text(operand, source)?;
        if let Some(import_dir) = file_ctx.imports.get(&operand_name) {
            return Some(ResolvedGoCall {
                symbol: field_name,
                destination: import_dir.clone(),
                receiver_type: None,
            });
        }

        if let Some(binding_type) = function_context.local_bindings.get(&operand_name) {
            let owner = simplify_go_type_name(binding_type);
            return Some(ResolvedGoCall {
                symbol: format!("{owner}.{field_name}"),
                destination: resolve_go_type_path(binding_type, file_ctx)
                    .unwrap_or_else(|| current_file.to_path_buf()),
                receiver_type: Some(owner),
            });
        }
    }

    let operand_text = node_text(operand, source)?;
    let parts: Vec<&str> = operand_text.split('.').collect();
    if parts.len() != 2 {
        return None;
    }

    let receiver_name = parts[0];
    let field_access = parts[1];
    let receiver_type = function_context.local_bindings.get(receiver_name)?;
    let target_type = file_ctx
        .struct_fields
        .get(&simplify_go_type_name(receiver_type))?
        .get(field_access)?;
    let owner = simplify_go_type_name(target_type);

    Some(ResolvedGoCall {
        symbol: format!("{owner}.{field_name}"),
        destination: resolve_go_type_path(target_type, file_ctx)
            .unwrap_or_else(|| current_file.to_path_buf()),
        receiver_type: Some(owner),
    })
}

fn collect_local_bindings(node: Node, source: &[u8]) -> HashMap<String, String> {
    let mut bindings = HashMap::new();

    for index in 0..node.named_child_count() {
        let Some(child) = node.named_child(index) else {
            continue;
        };
        if child.kind() != "parameter_list" {
            continue;
        }

        for inner in 0..child.named_child_count() {
            let Some(parameter) = child.named_child(inner) else {
                continue;
            };
            if parameter.kind() != "parameter_declaration" {
                continue;
            }

            let Some(parameter_type) = extract_parameter_type(parameter, source) else {
                continue;
            };

            for name in extract_parameter_names(parameter, source) {
                bindings.insert(name, parameter_type.clone());
            }
        }
    }

    bindings
}

fn extract_parameter_names(node: Node, source: &[u8]) -> Vec<String> {
    let mut names = Vec::new();

    for index in 0..node.named_child_count() {
        let Some(child) = node.named_child(index) else {
            continue;
        };
        if child.kind() == "identifier" {
            if let Some(name) = node_text(child, source) {
                names.push(name);
            }
        }
    }

    names
}

fn extract_parameter_type(node: Node, source: &[u8]) -> Option<String> {
    for index in 0..node.named_child_count() {
        let child = node.named_child(index)?;
        if matches!(
            child.kind(),
            "type_identifier" | "qualified_type" | "pointer_type"
        ) {
            return node_text(child, source);
        }
    }

    None
}

fn collect_imports(
    node: Node,
    source: &[u8],
    workspace_root: Option<&Path>,
    module_name: Option<&str>,
    imports: &mut HashMap<String, PathBuf>,
) {
    for index in 0..node.named_child_count() {
        let Some(child) = node.named_child(index) else {
            continue;
        };
        if child.kind() != "import_spec" {
            continue;
        }

        let path_node = child.child_by_field_name("path").or_else(|| {
            (0..child.named_child_count())
                .filter_map(|index| child.named_child(index))
                .find(|candidate| candidate.kind() == "interpreted_string_literal")
        });
        let alias_node = child.child_by_field_name("name").or_else(|| {
            (0..child.named_child_count())
                .filter_map(|index| child.named_child(index))
                .find(|candidate| candidate.kind() == "package_identifier")
        });
        let Some(path_text) = path_node.and_then(|n| node_text(n, source)) else {
            continue;
        };
        let import_path = path_text.trim_matches('"');
        let alias = alias_node
            .and_then(|n| node_text(n, source))
            .unwrap_or_else(|| {
                import_path
                    .rsplit('/')
                    .next()
                    .unwrap_or(import_path)
                    .to_string()
            });

        if let Some(root) = workspace_root {
            if let Some(dir) = import_path_to_dir(root, module_name, import_path) {
                imports.insert(alias, dir);
            }
        }
    }
}

fn collect_struct_fields(
    node: Node,
    source: &[u8],
    struct_fields: &mut HashMap<String, HashMap<String, String>>,
) {
    for index in 0..node.named_child_count() {
        let Some(child) = node.named_child(index) else {
            continue;
        };
        if child.kind() != "type_spec" {
            continue;
        }
        let Some(name_node) = child.child_by_field_name("name") else {
            continue;
        };
        let type_node = child.child_by_field_name("type").or_else(|| {
            (0..child.named_child_count())
                .filter_map(|inner| child.named_child(inner))
                .find(|candidate| candidate.kind() != "type_identifier")
        });
        let Some(type_node) = type_node else {
            continue;
        };
        if type_node.kind() != "struct_type" {
            continue;
        }
        let Some(type_name) = node_text(name_node, source) else {
            continue;
        };
        let fields = extract_struct_field_map(type_node, source);
        struct_fields.insert(type_name, fields);
    }
}

fn extract_struct_field_map(node: Node, source: &[u8]) -> HashMap<String, String> {
    let mut result = HashMap::new();
    for index in 0..node.named_child_count() {
        let Some(child) = node.named_child(index) else {
            continue;
        };
        if child.kind() != "field_declaration_list" {
            continue;
        }
        for field_index in 0..child.named_child_count() {
            let Some(field) = child.named_child(field_index) else {
                continue;
            };
            if field.kind() != "field_declaration" {
                continue;
            }
            let mut names = Vec::new();
            let mut field_type = None;
            for item_index in 0..field.named_child_count() {
                let Some(item) = field.named_child(item_index) else {
                    continue;
                };
                match item.kind() {
                    "field_identifier" => {
                        if let Some(name) = node_text(item, source) {
                            names.push(name);
                        }
                    }
                    "type_identifier" | "qualified_type" | "pointer_type" => {
                        field_type = node_text(item, source);
                    }
                    _ => {}
                }
            }
            if let Some(field_type) = field_type {
                for name in names {
                    result.insert(name, field_type.clone());
                }
            }
        }
    }
    result
}

pub(super) fn extract_receiver_type(receiver: Node, source: &[u8]) -> Option<String> {
    for index in 0..receiver.named_child_count() {
        let child = receiver.named_child(index)?;
        if child.kind() == "parameter_declaration" {
            for inner in 0..child.named_child_count() {
                let candidate = child.named_child(inner)?;
                if matches!(
                    candidate.kind(),
                    "type_identifier" | "qualified_type" | "pointer_type"
                ) {
                    return node_text(candidate, source).map(|value| simplify_go_type_name(&value));
                }
            }
        }
        if matches!(
            child.kind(),
            "type_identifier" | "qualified_type" | "pointer_type"
        ) {
            return node_text(child, source).map(|value| simplify_go_type_name(&value));
        }
    }
    node_text(receiver, source).map(|value| simplify_go_type_name(&value))
}

pub(super) fn resolve_go_type_path(type_name: &str, file_ctx: &GoFileContext) -> Option<PathBuf> {
    let normalized = type_name.trim().trim_start_matches('*');
    let (package_alias, _) = normalized.split_once('.')?;
    file_ctx.imports.get(package_alias).cloned()
}
