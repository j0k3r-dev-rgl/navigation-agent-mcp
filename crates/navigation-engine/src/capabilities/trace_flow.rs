use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::OnceLock;

use crate::analyzers::types::FindCalleesQuery;
use crate::analyzers::AnalyzerLanguage;
use crate::analyzers::AnalyzerRegistry;
use crate::error::EngineError;
use crate::protocol::{
    CalleeItem, EngineRequest, EngineResponse, TraceFlowRequestPayload, TraceFlowResult,
    TraceSymbolItem,
};
use crate::workspace::{canonicalize_workspace_root, contains_hard_ignored_segment, resolve_scope};
use tree_sitter::{Node, Parser};

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

static JAVA_INDEX: OnceLock<Option<JavaProjectIndex>> = OnceLock::new();

impl JavaProjectIndex {
    /// Get or create the Java project index (cached globally)
    pub fn get_or_create(workspace_root: &std::path::Path) -> Option<&'static Self> {
        JAVA_INDEX
            .get_or_init(|| {
                let mut index = Self::new_empty();
                index.scan_project(workspace_root);
                if index.is_empty() {
                    None
                } else {
                    Some(index)
                }
            })
            .as_ref()
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
                eprintln!(
                    "DEBUG: Interface '{}' has unknown package, file: {:?}",
                    class_name, file_path
                );
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
        eprintln!(
            "DEBUG find_implementations: looking for '{}'",
            interface_name
        );
        eprintln!(
            "DEBUG: interface_implementations has {} entries",
            self.interface_implementations.len()
        );
        eprintln!(
            "DEBUG: class_simple_to_fq has {} entries",
            self.class_simple_to_fq.len()
        );
        eprintln!(
            "DEBUG: interface_names has {} entries",
            self.interface_names.len()
        );

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
            items: vec![TraceSymbolItem {
                path: scope.public_path.clone(),
                language: Some(payload.analyzer_language.clone()),
            }],
            total_matched: 0,
            truncated: false,
            callees: vec![],
        });
    }

    // Use the absolute path from scope for the starting file
    let start_file_path = scope.absolute_path.to_string_lossy().to_string();

    // Determine language and whether to use Java index
    let is_java_file = start_file_path.ends_with(".java");
    let java_index = if is_java_file {
        JavaProjectIndex::get_or_create(&workspace_root)
    } else {
        None
    };

    // Now do recursive callee tracing
    let max_depth = payload.max_depth.unwrap_or(5) as usize;
    let mut visited = BTreeMap::new();
    let mut all_callees: Vec<CalleeItem> = Vec::new();

    trace_callees_recursive(
        &workspace_root,
        &start_file_path,
        &payload.symbol,
        0,
        max_depth,
        &mut visited,
        &mut all_callees,
        java_index,
    );

    // Dedupe callees by path and line
    let unique_callees: Vec<CalleeItem> = all_callees
        .into_iter()
        .fold(BTreeMap::new(), |mut acc, callee| {
            let key = (callee.path.clone(), callee.line, callee.callee.clone());
            acc.entry(key).or_insert(callee);
            acc
        })
        .into_values()
        .collect();

    let items = unique_callees
        .iter()
        .map(|c| TraceSymbolItem {
            path: c.path.clone(),
            language: c.language.clone(),
        })
        .collect();

    Ok(TraceFlowResult {
        resolved_path: Some(scope.public_path),
        total_matched: unique_callees.len(),
        truncated: false,
        items,
        callees: unique_callees,
    })
}

fn trace_callees_recursive(
    workspace_root: &std::path::Path,
    file_path: &str,
    symbol: &str,
    current_depth: usize,
    max_depth: usize,
    visited: &mut BTreeMap<(String, String), usize>,
    results: &mut Vec<CalleeItem>,
    java_index: Option<&'static JavaProjectIndex>,
) {
    if current_depth >= max_depth {
        return;
    }

    // Check if we've already visited this (file, symbol) pair at same or deeper depth
    let key = (file_path.to_string(), symbol.to_string());
    if let Some(prev_depth) = visited.get(&key) {
        if *prev_depth <= current_depth {
            return;
        }
    }
    visited.insert(key, current_depth);

    // Resolve file path - if relative, join with workspace root
    let absolute_file_path = if std::path::Path::new(file_path).is_absolute() {
        std::path::PathBuf::from(file_path)
    } else {
        workspace_root.join(file_path)
    };

    // Read the source file
    let source = match std::fs::read_to_string(&absolute_file_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "DEBUG: Failed to read file '{}': {}",
                absolute_file_path.display(),
                e
            );
            return;
        }
    };

    // Find the analyzer for this file type using registry with Auto language detection
    let registry = AnalyzerRegistry::new();
    let analyzer = match registry.analyzer_for_file(AnalyzerLanguage::Auto, &absolute_file_path) {
        Some(a) => a,
        None => {
            eprintln!(
                "DEBUG: No analyzer found for file '{}'",
                absolute_file_path.display()
            );
            return;
        }
    };

    eprintln!(
        "DEBUG: Analyzing file '{}' for symbol '{}'",
        absolute_file_path.display(),
        symbol
    );

    let query = FindCalleesQuery {
        source_path: absolute_file_path.clone(),
        target_symbol: symbol.to_string(),
    };

    let callees = analyzer.find_callees(&absolute_file_path, &source, &query);

    for callee in callees {
        let callee_item = CalleeItem {
            path: callee.path.clone(),
            line: callee.line,
            end_line: callee.end_line,
            column: callee.column,
            callee: callee.callee.clone(),
            callee_symbol: callee.callee_symbol.clone(),
            relation: callee.relation,
            language: callee.language.clone(),
            snippet: callee.snippet,
            depth: current_depth as u32 + 1,
            call_chain: vec![],
        };

        results.push(callee_item);

        // Recursively trace callees from referenced files
        // Callee paths are relative to the file they were found in, so resolve them
        let callee_file_path = if std::path::Path::new(&callee.path).is_absolute() {
            std::path::PathBuf::from(&callee.path)
        } else {
            // Resolve relative to the current file's directory
            absolute_file_path
                .parent()
                .map(|p| p.join(&callee.path))
                .unwrap_or_else(|| workspace_root.join(&callee.path))
        };

        // Check if the callee's receiver type is an interface and trace implementations
        let mut traced_interfaces = false;
        if let (Some(index), Some(receiver_type)) = (java_index, &callee.receiver_type) {
            // Clean up the receiver type (remove generics, etc.)
            let clean_type = receiver_type
                .split('<')
                .next()
                .unwrap_or(receiver_type)
                .trim();

            if index.is_interface(clean_type) {
                // Find implementations of this interface
                let implementations = index.find_implementations(clean_type);
                eprintln!(
                    "DEBUG: Receiver '{}' is an interface with {} implementations",
                    clean_type,
                    implementations.len()
                );

                if !implementations.is_empty() {
                    traced_interfaces = true;

                    // Trace each implementation
                    for impl_path in &implementations {
                        eprintln!(
                            "DEBUG: Tracing implementation '{}' for interface '{}'",
                            impl_path, clean_type
                        );
                        trace_callees_recursive(
                            workspace_root,
                            impl_path,
                            &callee.callee,
                            current_depth + 1,
                            max_depth,
                            visited,
                            results,
                            Some(index),
                        );
                    }
                }
            }
        }

        // Trace the callee directly if it's in a different file and wasn't an interface
        if callee_file_path.to_string_lossy() != absolute_file_path.to_string_lossy()
            && !traced_interfaces
        {
            trace_callees_recursive(
                workspace_root,
                &callee_file_path.to_string_lossy(),
                &callee.callee,
                current_depth + 1,
                max_depth,
                visited,
                results,
                java_index,
            );
        }
    }
}
