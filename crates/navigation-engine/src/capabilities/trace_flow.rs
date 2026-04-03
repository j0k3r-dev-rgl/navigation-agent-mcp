use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::analyzers::types::FindCalleesQuery;
use crate::analyzers::AnalyzerLanguage;
use crate::analyzers::AnalyzerRegistry;
use crate::error::EngineError;
use crate::protocol::{
    EngineRequest, EngineResponse, FindSymbolRequestPayload, TraceFlowLineRange, TraceFlowNode,
    TraceFlowRequestPayload, TraceFlowResult, TraceFlowVia,
};
use crate::workspace::{
    canonicalize_workspace_root, contains_hard_ignored_segment, public_path, resolve_scope,
};
use tree_sitter::{Node, Parser};

use super::find_symbol::find_symbol;

pub const CAPABILITY: &str = "workspace.trace_flow";

/// Index of Java interfaces and their implementations in the project
#[allow(dead_code)]
pub struct JavaProjectIndex {
    /// Map: interface fully qualified name -> list of implementation file paths
    interface_implementations: HashMap<String, Vec<String>>,
    /// Map: interface fully qualified name -> interface file path
    interface_paths: HashMap<String, String>,
    /// Map: class simple name -> fully qualified name (from imports/context)
    class_simple_to_fq: HashMap<String, String>,
    /// Map: field name -> (type, file path) for fields whose type is an interface
    interface_field_types: HashMap<String, HashMap<String, (String, String)>>,
    /// Set of all interface names in the project (simple names)
    interface_names: HashSet<String>,
    /// Map: file path -> (package, imports, class name)
    file_contexts: HashMap<String, JavaFileContextInfo>,
    /// Project prefix extracted from packages
    project_prefix: Option<String>,
}

/// File context information extracted from a Java file
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct JavaFileContextInfo {
    package: String,
    imports: HashMap<String, String>,
    class_name: String,
    implements_interfaces: Vec<String>,
    is_interface: bool,
}

impl JavaProjectIndex {
    /// Build the Java project index for the given workspace.
    pub fn build(workspace_root: &std::path::Path) -> Option<Self> {
        let mut index = Self::new_empty();
        index.scan_project(workspace_root);
        if index.is_empty() {
            None
        } else {
            Some(index)
        }
    }

    fn new_empty() -> Self {
        Self {
            interface_implementations: HashMap::new(),
            interface_paths: HashMap::new(),
            class_simple_to_fq: HashMap::new(),
            interface_field_types: HashMap::new(),
            interface_names: HashSet::new(),
            file_contexts: HashMap::new(),
            project_prefix: None,
        }
    }

    fn is_empty(&self) -> bool {
        self.interface_names.is_empty()
    }

    /// Scan all Java files in the project to find interface implementations
    fn scan_project(&mut self, workspace_root: &std::path::Path) {
        use walkdir::WalkDir;

        // First pass: collect all interfaces
        let mut java_files = 0;
        for entry in WalkDir::new(workspace_root)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "java") {
                java_files += 1;
                if let Ok(source) = std::fs::read_to_string(path) {
                    self.extract_interface_info(path, &source);
                }
            }
        }
        eprintln!(
            "DEBUG: First pass complete. Found {} Java files",
            java_files
        );

        // Second pass: find implementations of interfaces
        for entry in WalkDir::new(workspace_root)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "java") {
                if let Ok(source) = std::fs::read_to_string(path) {
                    self.extract_implementations(path, &source, workspace_root);
                }
            }
        }
        eprintln!(
            "DEBUG: Second pass complete. Found {} interfaces and {} implementations",
            self.interface_paths.len(),
            self.interface_implementations.len()
        );
    }

    /// Extract interface information from a Java file
    fn extract_interface_info(&mut self, file_path: &std::path::Path, source: &str) {
        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_java::LANGUAGE.into())
            .is_err()
        {
            return;
        };

        let tree = match parser.parse(source, None) {
            Some(t) => t,
            None => return,
        };

        let root = tree.root_node();
        let mut package_name = String::new();
        let mut imports: HashMap<String, String> = HashMap::new();
        let mut class_name = String::new();
        let mut implements_interfaces: Vec<String> = Vec::new();
        let mut is_interface = false;

        // Extract package
        for index in 0..root.named_child_count() {
            if let Some(child) = root.named_child(index) {
                match child.kind() {
                    "package_declaration" => {
                        // Look for scoped_identifier child which contains the package name
                        for p_index in 0..child.named_child_count() {
                            if let Some(pkg_child) = child.named_child(p_index) {
                                if pkg_child.kind() == "scoped_identifier"
                                    || pkg_child.kind() == "identifier"
                                {
                                    if let Some(name) =
                                        java_node_text(&pkg_child, source.as_bytes())
                                    {
                                        package_name = name;
                                        if self.project_prefix.is_none() {
                                            self.project_prefix =
                                                extract_project_prefix(&package_name);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    "import_declaration" => {
                        if let Some((simple, full)) = extract_import_info(&child, source.as_bytes())
                        {
                            imports.insert(simple, full);
                        }
                    }
                    "class_declaration" => {
                        if let Some(name_node) = child.child_by_field_name("name") {
                            if let Some(name) = java_node_text(&name_node, source.as_bytes()) {
                                class_name.clone_from(&name);
                            }
                        }
                        // Extract implements clause
                        self.extract_implements(
                            &child,
                            source.as_bytes(),
                            &imports,
                            &mut implements_interfaces,
                        );
                    }
                    "interface_declaration" => {
                        is_interface = true;
                        if let Some(name_node) = child.child_by_field_name("name") {
                            if let Some(name) = java_node_text(&name_node, source.as_bytes()) {
                                class_name.clone_from(&name);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        if package_name.is_empty() {
            package_name = "unknown".to_string();
        }

        if !class_name.is_empty() {
            let fq_name = if package_name == "unknown" {
                class_name.clone()
            } else {
                format!("{}.{}", package_name, class_name)
            };

            // If this is an interface, store it
            if is_interface {
                self.interface_names.insert(class_name.clone());
                self.interface_paths.insert(
                    fq_name.clone(),
                    file_path.to_string_lossy().replace('\\', "/"),
                );

                // Also store simple name to FQ mapping
                self.class_simple_to_fq.insert(class_name.clone(), fq_name);
            }
        }

        // Store file context
        let file_str = file_path.to_string_lossy().replace('\\', "/");
        self.file_contexts.insert(
            file_str,
            JavaFileContextInfo {
                package: package_name,
                imports,
                class_name,
                implements_interfaces,
                is_interface,
            },
        );
    }

    /// Extract implements clause from a class declaration
    fn extract_implements(
        &self,
        class_node: &Node,
        source: &[u8],
        imports: &HashMap<String, String>,
        implements_list: &mut Vec<String>,
    ) {
        // Look for "implements" in the class declaration
        // The implements clause comes after the extends clause (if present)
        // tree-sitter-java: class_declaration has children including type_list for implements

        for i in 0..class_node.child_count() {
            if let Some(child) = class_node.child(i) {
                if child.kind() == "super_interfaces" {
                    // super_interfaces contains the implemented interfaces
                    // It can contain type_list (for multiple interfaces) or a single type
                    for j in 0..child.named_child_count() {
                        if let Some(type_node) = child.named_child(j) {
                            match type_node.kind() {
                                "type_identifier" | "scoped_type_identifier" => {
                                    if let Some(type_name) = java_node_text(&type_node, source) {
                                        let fq_name = self.resolve_type_name(&type_name, imports);
                                        implements_list.push(fq_name);
                                    }
                                }
                                "type_list" => {
                                    // Multiple interfaces: type_list contains type_identifiers
                                    for k in 0..type_node.named_child_count() {
                                        if let Some(inner) = type_node.named_child(k) {
                                            if let Some(type_name) = java_node_text(&inner, source)
                                            {
                                                let fq_name =
                                                    self.resolve_type_name(&type_name, imports);
                                                implements_list.push(fq_name);
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    break;
                }
            }
        }
    }

    /// Extract implementations from a Java file (second pass)
    #[allow(unused_variables)]
    fn extract_implementations(
        &mut self,
        file_path: &std::path::Path,
        _source: &str,
        _workspace_root: &std::path::Path,
    ) {
        // Get file context from first pass
        let file_str = file_path.to_string_lossy().replace('\\', "/");
        let file_ctx = match self.file_contexts.get(&file_str) {
            Some(ctx) => ctx,
            None => return,
        };

        // If this class implements interfaces, register the mappings
        if !file_ctx.implements_interfaces.is_empty() && !file_ctx.is_interface {
            // Note: class_fq could be used in future for tracking which class implements which interface
            let _class_fq = format!("{}.{}", file_ctx.package, file_ctx.class_name);

            for interface_fq in &file_ctx.implements_interfaces {
                // Always register the implementation (don't require interface to be known first)
                eprintln!(
                    "DEBUG: Registering implementation '{}' for interface '{}'",
                    file_str, interface_fq
                );
                self.interface_implementations
                    .entry(interface_fq.clone())
                    .or_default()
                    .push(file_str.clone());
            }
        }
    }

    /// Resolve a type name to fully qualified name using imports
    fn resolve_type_name(&self, type_name: &str, imports: &HashMap<String, String>) -> String {
        // Remove generic parameters
        let base_type = type_name.split('<').next().unwrap_or(type_name).trim();

        // Check imports first
        if let Some(fq) = imports.get(base_type) {
            return fq.clone();
        }

        // If it's already a known interface, return it
        if self.interface_paths.contains_key(base_type) {
            return base_type.to_string();
        }

        // Try to construct FQN from simple name
        if let Some(_project_prefix) = &self.project_prefix {
            // Could be in project - return as-is for now
            base_type.to_string()
        } else {
            base_type.to_string()
        }
    }

    /// Find all implementations of an interface (by fully qualified or simple name)
    pub fn find_implementations(&self, interface_name: &str) -> Vec<String> {
        // Try FQN first
        if let Some(imps) = self.interface_implementations.get(interface_name) {
            return imps.clone();
        }

        // Try simple name
        if let Some(fq) = self.class_simple_to_fq.get(interface_name) {
            if let Some(imps) = self.interface_implementations.get(fq) {
                return imps.clone();
            }
        }

        // Try matching by simple name against interface_names
        if self.interface_names.contains(interface_name) {
            if let Some(fq) = self.class_simple_to_fq.get(interface_name) {
                if let Some(imps) = self.interface_implementations.get(fq) {
                    return imps.clone();
                }
            }
        }

        Vec::new()
    }

    /// Check if a type name is an interface
    pub fn is_interface(&self, type_name: &str) -> bool {
        // Check by simple name
        if self.interface_names.contains(type_name) {
            return true;
        }

        // Check by FQN
        self.interface_paths.contains_key(type_name)
    }

    /// Get the file path for an interface
    #[allow(dead_code)]
    pub fn get_interface_path(&self, interface_name: &str) -> Option<String> {
        // Try FQN first
        if let Some(path) = self.interface_paths.get(interface_name) {
            return Some(path.clone());
        }

        // Try convert simple to FQN
        self.class_simple_to_fq
            .get(interface_name)
            .and_then(|fq| self.interface_paths.get(fq).cloned())
    }

    /// Resolve a field's type in a file context
    #[allow(dead_code)]
    pub fn resolve_field_type(&self, field_name: &str, file_path: &str) -> Option<String> {
        self.interface_field_types
            .get(file_path)
            .and_then(|fields| fields.get(field_name))
            .map(|(ty, _)| ty.clone())
    }
}

/// Extract project prefix from package name (first 3 segments typically)
fn extract_project_prefix(package: &str) -> Option<String> {
    let segments: Vec<&str> = package.split('.').take(3).collect();
    if segments.len() >= 3 {
        Some(segments.join("."))
    } else {
        None
    }
}

/// Extract import information: (simple_name, fully_qualified_name)
fn extract_import_info(node: &Node, source: &[u8]) -> Option<(String, String)> {
    let name_node = node
        .named_children(&mut node.walk())
        .find(|c| c.kind() == "scoped_identifier" || c.kind() == "identifier")?;

    let full_name = java_node_text(&name_node, source)?;
    let simple_name = full_name
        .split('.')
        .last()
        .unwrap_or(&full_name)
        .to_string();

    Some((simple_name, full_name))
}

/// Get text from a tree-sitter node
fn java_node_text(node: &Node, source: &[u8]) -> Option<String> {
    node.utf8_text(source)
        .ok()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

/// Check if a file is an infrastructure/persistence file that should stop recursion
pub fn is_infrastructure_file(path: &str) -> bool {
    let path_lower = path.to_lowercase();
    // Check if path contains infrastructure/persistence
    let is_infrastructure_path = path_lower.contains("infrastructure/persistence");
    // Check if file ends with infrastructure patterns
    let is_infrastructure_file = path_lower.ends_with("adapter.java")
        || path_lower.ends_with("repository.java")
        || path_lower.ends_with("dao.java");
    is_infrastructure_path || is_infrastructure_file
}

impl JavaProjectIndex {}

pub fn handle(request: EngineRequest) -> EngineResponse {
    let parsed_payload = serde_json::from_value::<TraceFlowRequestPayload>(request.payload.clone());

    match parsed_payload {
        Ok(payload) => match trace_flow(&request.workspace_root, payload) {
            Ok(result) => EngineResponse::success(request.id, &result),
            Err(error) => EngineResponse::error(request.id, error),
        },
        Err(error) => {
            EngineResponse::error(request.id, EngineError::invalid_request(error.to_string()))
        }
    }
}

pub fn trace_flow(
    workspace_root: &str,
    payload: TraceFlowRequestPayload,
) -> Result<TraceFlowResult, EngineError> {
    let workspace_root = canonicalize_workspace_root(workspace_root)?;
    let scope = resolve_scope(&workspace_root, Some(payload.path.as_str()))?;

    if !scope.absolute_path.is_file() {
        return Err(EngineError::file_not_found(payload.path.as_str()));
    }

    if contains_hard_ignored_segment(&workspace_root, &scope.absolute_path) {
        return Ok(TraceFlowResult {
            resolved_path: Some(scope.public_path.clone()),
            truncated: false,
            root: None,
        });
    }

    let start_file_path = scope.absolute_path.to_string_lossy().to_string();
    let is_java_file = start_file_path.ends_with(".java");
    let java_index = if is_java_file {
        JavaProjectIndex::build(&workspace_root)
    } else {
        None
    };
    let max_depth = payload.max_depth.unwrap_or(5) as usize;
    let analyzer_language = parse_analyzer_language(&payload.analyzer_language)?;
    let mut branch_visited = BTreeSet::new();
    let root = build_execution_tree(
        &workspace_root,
        &scope.absolute_path,
        &payload.symbol,
        analyzer_language,
        java_index.as_ref(),
        0,
        max_depth,
        None,
        &mut branch_visited,
    )?;

    Ok(TraceFlowResult {
        resolved_path: Some(scope.public_path),
        truncated: false,
        root,
    })
}

fn build_execution_tree(
    workspace_root: &std::path::Path,
    file_path: &std::path::Path,
    symbol: &str,
    analyzer_language: AnalyzerLanguage,
    java_index: Option<&JavaProjectIndex>,
    current_depth: usize,
    max_depth: usize,
    via: Option<Vec<TraceFlowVia>>,
    branch_visited: &mut BTreeSet<(String, String)>,
) -> Result<Option<TraceFlowNode>, EngineError> {
    let absolute_file = if file_path.is_absolute() {
        file_path.to_path_buf()
    } else {
        workspace_root.join(file_path)
    };

    let resolved = match absolute_file.canonicalize() {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };

    let visit_key = (resolved.to_string_lossy().to_string(), symbol.to_string());
    let Some(metadata) =
        resolve_symbol_metadata(workspace_root, &resolved, symbol, analyzer_language)?
    else {
        return Ok(None);
    };

    if branch_visited.contains(&visit_key) {
        return Ok(Some(TraceFlowNode {
            symbol: qualify_symbol_name(java_index, &resolved, &metadata.symbol),
            path: public_path(workspace_root, &resolved),
            kind: classify_trace_node_kind(&resolved, java_index),
            range_line: TraceFlowLineRange {
                init: metadata.line,
                end: metadata.line_end,
            },
            via,
            callers: Vec::new(),
        }));
    }

    branch_visited.insert(visit_key.clone());

    let mut node = TraceFlowNode {
        symbol: qualify_symbol_name(java_index, &resolved, &metadata.symbol),
        path: public_path(workspace_root, &resolved),
        kind: classify_trace_node_kind(&resolved, java_index),
        range_line: TraceFlowLineRange {
            init: metadata.line,
            end: metadata.line_end,
        },
        via,
        callers: Vec::new(),
    };

    let should_expand = current_depth < max_depth && !is_infrastructure_file(&node.path);
    if should_expand {
        let children = collect_child_nodes(
            workspace_root,
            &resolved,
            symbol,
            analyzer_language,
            java_index,
            current_depth,
            max_depth,
            branch_visited,
        )?;
        node.callers = children;
    }

    sort_trace_node(&mut node);

    branch_visited.remove(&visit_key);
    Ok(Some(node))
}

fn collect_child_nodes(
    workspace_root: &std::path::Path,
    file_path: &std::path::Path,
    symbol: &str,
    analyzer_language: AnalyzerLanguage,
    java_index: Option<&JavaProjectIndex>,
    current_depth: usize,
    max_depth: usize,
    branch_visited: &mut BTreeSet<(String, String)>,
) -> Result<Vec<TraceFlowNode>, EngineError> {
    let source = match std::fs::read_to_string(file_path) {
        Ok(value) => value,
        Err(_) => return Ok(Vec::new()),
    };

    let registry = AnalyzerRegistry::new();
    let Some(analyzer) = registry.analyzer_for_file(AnalyzerLanguage::Auto, file_path) else {
        return Ok(Vec::new());
    };

    let query = FindCalleesQuery {
        target_symbol: symbol.to_string(),
    };

    let callees = analyzer.find_callees(file_path, &source, &query);
    let mut grouped: BTreeMap<(String, String), TraceFlowNode> = BTreeMap::new();

    for callee in callees {
        let resolved_call = resolve_callee_targets(
            workspace_root,
            file_path,
            &callee,
            analyzer_language,
            java_index,
        )?;

        let has_targets = resolved_call.interface_target.is_some()
            || !resolved_call.implementation_targets.is_empty()
            || !resolved_call.direct_targets.is_empty();

        if !has_targets {
            if let Some(fallback) =
                build_unresolved_leaf(workspace_root, file_path, &callee, analyzer_language)
            {
                merge_trace_node(&mut grouped, fallback);
            }
            continue;
        }

        let via = vec![TraceFlowVia {
            line: callee.line,
            column: callee.column,
            snippet: callee.snippet.clone().map(|s| compact_snippet(&s)),
            receiver_type: callee.receiver_type.clone(),
        }];

        if let Some(interface_target) = resolved_call.interface_target {
            if let Some(child) = build_execution_tree(
                workspace_root,
                &interface_target.path,
                &interface_target.symbol,
                analyzer_language,
                java_index,
                current_depth + 1,
                max_depth,
                Some(via.clone()),
                branch_visited,
            )? {
                let mut child = child;
                for implementation_target in resolved_call.implementation_targets {
                    if let Some(implementation_node) = build_execution_tree(
                        workspace_root,
                        &implementation_target.path,
                        &implementation_target.symbol,
                        analyzer_language,
                        java_index,
                        current_depth + 2,
                        max_depth,
                        Some(via.clone()),
                        branch_visited,
                    )? {
                        merge_child_node(&mut child, implementation_node);
                    }
                }
                merge_trace_node(&mut grouped, child);
            }
            continue;
        }

        for target in resolved_call.direct_targets {
            if let Some(child) = build_execution_tree(
                workspace_root,
                &target.path,
                &target.symbol,
                analyzer_language,
                java_index,
                current_depth + 1,
                max_depth,
                Some(via.clone()),
                branch_visited,
            )? {
                merge_trace_node(&mut grouped, child);
            }
        }
    }

    let mut nodes: Vec<_> = grouped.into_values().collect();
    nodes.sort_by(compare_trace_nodes);
    Ok(nodes)
}

fn build_unresolved_leaf(
    workspace_root: &std::path::Path,
    current_file: &std::path::Path,
    callee: &crate::analyzers::types::CalleeDefinition,
    analyzer_language: AnalyzerLanguage,
) -> Option<TraceFlowNode> {
    if analyzer_language == AnalyzerLanguage::Java {
        return None;
    }

    Some(TraceFlowNode {
        symbol: callee.callee.clone(),
        path: public_path(workspace_root, current_file),
        kind: classify_trace_node_kind(current_file, None),
        range_line: TraceFlowLineRange {
            init: callee.line,
            end: callee.end_line,
        },
        via: Some(vec![TraceFlowVia {
            line: callee.line,
            column: callee.column,
            snippet: callee.snippet.clone().map(|s| compact_snippet(&s)),
            receiver_type: callee.receiver_type.clone(),
        }]),
        callers: Vec::new(),
    })
}

#[derive(Debug, Clone)]
struct ResolvedTraceTarget {
    path: std::path::PathBuf,
    symbol: String,
}

struct ResolvedTraceCall {
    interface_target: Option<ResolvedTraceTarget>,
    implementation_targets: Vec<ResolvedTraceTarget>,
    direct_targets: Vec<ResolvedTraceTarget>,
}

fn resolve_callee_targets(
    workspace_root: &std::path::Path,
    current_file: &std::path::Path,
    callee: &crate::analyzers::types::CalleeDefinition,
    analyzer_language: AnalyzerLanguage,
    java_index: Option<&JavaProjectIndex>,
) -> Result<ResolvedTraceCall, EngineError> {
    let mut interface_target = None;
    let mut implementation_targets = Vec::new();
    let mut direct_targets = Vec::new();

    if analyzer_language == AnalyzerLanguage::Java {
        if let (Some(index), Some(receiver_type)) = (java_index, &callee.receiver_type) {
            let clean_type = receiver_type
                .split('<')
                .next()
                .unwrap_or(receiver_type)
                .trim();
            if index.is_interface(clean_type) {
                if let Some(interface_path) = index.get_interface_path(clean_type) {
                    interface_target = Some(ResolvedTraceTarget {
                        path: std::path::PathBuf::from(interface_path),
                        symbol: callee.callee.clone(),
                    });
                }

                for impl_path in index.find_implementations(clean_type) {
                    implementation_targets.push(ResolvedTraceTarget {
                        path: std::path::PathBuf::from(impl_path),
                        symbol: callee.callee.clone(),
                    });
                }

                return Ok(ResolvedTraceCall {
                    interface_target,
                    implementation_targets,
                    direct_targets,
                });
            }
        }
    }

    let same_file_target = resolve_symbol_metadata(
        workspace_root,
        current_file,
        &callee.callee,
        analyzer_language,
    )?;
    if same_file_target.is_some() {
        direct_targets.push(ResolvedTraceTarget {
            path: current_file.to_path_buf(),
            symbol: callee.callee.clone(),
        });
        return Ok(ResolvedTraceCall {
            interface_target,
            implementation_targets,
            direct_targets,
        });
    }

    let candidate_path = if std::path::Path::new(&callee.path).is_absolute() {
        std::path::PathBuf::from(&callee.path)
    } else {
        current_file
            .parent()
            .map(|parent| parent.join(&callee.path))
            .unwrap_or_else(|| workspace_root.join(&callee.path))
    };

    if candidate_path.is_dir() {
        if let Some(scoped_match) = resolve_symbol_in_scope(
            workspace_root,
            &candidate_path,
            &callee.callee,
            analyzer_language,
        )? {
            direct_targets.push(scoped_match);
        } else if let Some(global_match) =
            resolve_symbol_globally(workspace_root, &callee.callee, analyzer_language)?
        {
            direct_targets.push(global_match);
        } else {
            direct_targets.push(ResolvedTraceTarget {
                path: candidate_path,
                symbol: callee.callee.clone(),
            });
        }
    } else if resolve_symbol_metadata(
        workspace_root,
        &candidate_path,
        &callee.callee,
        analyzer_language,
    )?
    .is_some()
    {
        direct_targets.push(ResolvedTraceTarget {
            path: candidate_path,
            symbol: callee.callee.clone(),
        });
    } else if analyzer_language == AnalyzerLanguage::Go {
        if let Some(global_match) =
            resolve_symbol_globally(workspace_root, &callee.callee, analyzer_language)?
        {
            direct_targets.push(global_match);
        } else if candidate_path.exists() {
            direct_targets.push(ResolvedTraceTarget {
                path: candidate_path,
                symbol: callee.callee.clone(),
            });
        }
    } else if candidate_path.exists() {
        direct_targets.push(ResolvedTraceTarget {
            path: candidate_path,
            symbol: callee.callee.clone(),
        });
    }

    Ok(ResolvedTraceCall {
        interface_target,
        implementation_targets,
        direct_targets,
    })
}

fn resolve_symbol_globally(
    workspace_root: &std::path::Path,
    symbol: &str,
    analyzer_language: AnalyzerLanguage,
) -> Result<Option<ResolvedTraceTarget>, EngineError> {
    find_symbol_target(workspace_root, symbol, analyzer_language, None)
}

fn resolve_symbol_in_scope(
    workspace_root: &std::path::Path,
    scope_path: &std::path::Path,
    symbol: &str,
    analyzer_language: AnalyzerLanguage,
) -> Result<Option<ResolvedTraceTarget>, EngineError> {
    if let Some(exact_match) = find_symbol_target(
        workspace_root,
        symbol,
        analyzer_language,
        Some(public_path(workspace_root, scope_path)),
    )? {
        return Ok(Some(exact_match));
    }

    let Some((_, terminal_symbol)) = symbol.rsplit_once('.') else {
        return Ok(None);
    };

    find_symbol_target(
        workspace_root,
        terminal_symbol,
        analyzer_language,
        Some(public_path(workspace_root, scope_path)),
    )
}

fn find_symbol_target(
    workspace_root: &std::path::Path,
    symbol: &str,
    analyzer_language: AnalyzerLanguage,
    path: Option<String>,
) -> Result<Option<ResolvedTraceTarget>, EngineError> {
    let result = find_symbol(
        workspace_root.to_string_lossy().as_ref(),
        FindSymbolRequestPayload {
            symbol: symbol.to_string(),
            path,
            analyzer_language: analyzer_language_name(analyzer_language).to_string(),
            public_language_filter: None,
            kind: "any".to_string(),
            match_mode: "exact".to_string(),
            limit: 10,
        },
    )?;

    Ok(result
        .items
        .into_iter()
        .next()
        .map(|item| ResolvedTraceTarget {
            path: workspace_root.join(item.path),
            symbol: item.symbol,
        }))
}

#[derive(Debug, Clone)]
struct SymbolMetadata {
    symbol: String,
    line: u32,
    line_end: u32,
}

fn resolve_symbol_metadata(
    workspace_root: &std::path::Path,
    file_path: &std::path::Path,
    symbol: &str,
    analyzer_language: AnalyzerLanguage,
) -> Result<Option<SymbolMetadata>, EngineError> {
    let public = public_path(workspace_root, file_path);
    let result = find_symbol(
        workspace_root.to_string_lossy().as_ref(),
        FindSymbolRequestPayload {
            symbol: symbol.to_string(),
            path: Some(public),
            analyzer_language: analyzer_language_name(analyzer_language).to_string(),
            public_language_filter: None,
            kind: "any".to_string(),
            match_mode: "exact".to_string(),
            limit: 50,
        },
    )?;

    Ok(result
        .items
        .into_iter()
        .find(|item| symbol_matches_target(&item.symbol, symbol))
        .map(|item| SymbolMetadata {
            symbol: item.symbol,
            line: item.line,
            line_end: item.line_end,
        }))
}

fn symbol_matches_target(candidate: &str, target: &str) -> bool {
    candidate == target
        || candidate
            .rsplit_once('.')
            .map(|(_, suffix)| suffix == target)
            .unwrap_or(false)
}

fn merge_trace_node(
    grouped: &mut BTreeMap<(String, String), TraceFlowNode>,
    mut candidate: TraceFlowNode,
) {
    let key = (candidate.path.clone(), candidate.symbol.clone());
    if let Some(existing) = grouped.get_mut(&key) {
        merge_vias(existing, candidate.via.take());
        for child in candidate.callers.drain(..) {
            merge_child_node(existing, child);
        }
    } else {
        grouped.insert(key, candidate);
    }
}

fn merge_child_node(parent: &mut TraceFlowNode, child: TraceFlowNode) {
    if let Some(existing) = parent
        .callers
        .iter_mut()
        .find(|current| current.path == child.path && current.symbol == child.symbol)
    {
        merge_vias(existing, child.via);
        for nested in child.callers {
            merge_child_node(existing, nested);
        }
    } else {
        parent.callers.push(child);
        parent.callers.sort_by(compare_trace_nodes);
    }
}

fn merge_vias(node: &mut TraceFlowNode, incoming: Option<Vec<TraceFlowVia>>) {
    let Some(incoming) = incoming else {
        return;
    };

    let vias = node.via.get_or_insert_with(Vec::new);
    for via in incoming {
        if !vias.iter().any(|current| {
            current.line == via.line
                && current.column == via.column
                && current.snippet == via.snippet
                && current.receiver_type == via.receiver_type
        }) {
            vias.push(via);
        }
    }
    vias.sort_by(|left, right| {
        (left.line, left.column.unwrap_or(0)).cmp(&(right.line, right.column.unwrap_or(0)))
    });
}

fn sort_trace_node(node: &mut TraceFlowNode) {
    if let Some(vias) = node.via.as_mut() {
        vias.sort_by(|left, right| {
            (left.line, left.column.unwrap_or(0)).cmp(&(right.line, right.column.unwrap_or(0)))
        });
    }

    node.callers.sort_by(compare_trace_nodes);
    for child in &mut node.callers {
        sort_trace_node(child);
    }
}

fn compare_trace_nodes(left: &TraceFlowNode, right: &TraceFlowNode) -> std::cmp::Ordering {
    let left_line = left
        .via
        .as_ref()
        .and_then(|vias| vias.first())
        .map(|via| via.line)
        .unwrap_or(left.range_line.init);
    let right_line = right
        .via
        .as_ref()
        .and_then(|vias| vias.first())
        .map(|via| via.line)
        .unwrap_or(right.range_line.init);

    (left_line, left.range_line.init, &left.path, &left.symbol).cmp(&(
        right_line,
        right.range_line.init,
        &right.path,
        &right.symbol,
    ))
}

fn compact_snippet(snippet: &str) -> String {
    snippet.lines().next().unwrap_or("").trim().to_string()
}

fn classify_trace_node_kind(
    path: &std::path::Path,
    java_index: Option<&JavaProjectIndex>,
) -> String {
    if path.to_string_lossy().ends_with(".java") {
        let normalized = path.to_string_lossy().replace('\\', "/");
        if let Some(index) = java_index {
            if let Some(ctx) = index.file_contexts.get(&normalized) {
                if ctx.is_interface {
                    return "interface-method".to_string();
                }
                if !ctx.implements_interfaces.is_empty() {
                    return "implementation-method".to_string();
                }
                return "class-method".to_string();
            }
        }
        return "class-method".to_string();
    }

    "function".to_string()
}

fn qualify_symbol_name(
    java_index: Option<&JavaProjectIndex>,
    file_path: &std::path::Path,
    symbol: &str,
) -> String {
    if !file_path.to_string_lossy().ends_with(".java") {
        return symbol.to_string();
    }

    let Some(index) = java_index else {
        return symbol.to_string();
    };
    let key = file_path.to_string_lossy().replace('\\', "/");
    let Some(ctx) = index.file_contexts.get(&key) else {
        return symbol.to_string();
    };
    if ctx.class_name.is_empty() {
        symbol.to_string()
    } else {
        format!("{}#{}", ctx.class_name, symbol)
    }
}

fn analyzer_language_name(language: AnalyzerLanguage) -> &'static str {
    match language {
        AnalyzerLanguage::Auto => "auto",
        AnalyzerLanguage::Go => "go",
        AnalyzerLanguage::Java => "java",
        AnalyzerLanguage::Python => "python",
        AnalyzerLanguage::Rust => "rust",
        AnalyzerLanguage::Typescript => "typescript",
    }
}

fn parse_analyzer_language(value: &str) -> Result<AnalyzerLanguage, EngineError> {
    match value {
        "auto" => Ok(AnalyzerLanguage::Auto),
        "go" => Ok(AnalyzerLanguage::Go),
        "java" => Ok(AnalyzerLanguage::Java),
        "python" => Ok(AnalyzerLanguage::Python),
        "rust" => Ok(AnalyzerLanguage::Rust),
        "typescript" => Ok(AnalyzerLanguage::Typescript),
        other => Err(EngineError::invalid_request(format!(
            "Unsupported analyzerLanguage '{}'.",
            other
        ))),
    }
}
