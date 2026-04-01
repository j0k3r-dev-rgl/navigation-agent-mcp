use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use tree_sitter::{Node, Parser};

use super::language_analyzer::LanguageAnalyzer;
use super::types::{
    infer_public_language, normalize_public_endpoint_kind, normalize_public_symbol_kind,
    AnalyzerLanguage, CalleeDefinition, CallerDefinition, CallerTarget, EndpointDefinition,
    FindCalleesQuery, FindCallersQuery, FindEndpointsQuery, FindSymbolQuery, SymbolDefinition,
};

/// TypeScript file context with import resolution and local definitions tracking
struct TypeScriptFileContext {
    /// Map of imported names to their resolved status
    imports: HashMap<String, ResolvedImport>,
    /// Set of locally defined identifiers in this file (functions, variables, classes)
    local_definitions: std::collections::HashSet<String>,
    /// TsConfig paths mapping
    path_mappings: Vec<(String, Vec<String>)>,
    /// Base URL for relative imports
    base_url: Option<String>,
}

#[derive(Debug, Clone)]
enum ResolvedImport {
    /// Import from local project file
    Local { path: String },
    /// Import from node_modules (external)
    External { package: String },
    /// Could not resolve
    Unknown,
}

impl TypeScriptFileContext {
    fn new(
        path_mappings: Vec<(String, Vec<String>)>,
        base_url: Option<String>,
        local_definitions: std::collections::HashSet<String>,
    ) -> Self {
        Self {
            imports: HashMap::new(),
            local_definitions,
            path_mappings,
            base_url,
        }
    }

    /// Check if a callee (method call) is from the project
    /// Returns true if the callee is defined in the project (not external/framework)
    fn is_callee_from_project(&self, receiver: Option<&str>, callee_name: &str) -> bool {
        // Case 1: Member expression - receiver.object.method()
        if let Some(receiver_name) = receiver {
            // Check if receiver has an import
            if let Some(resolved) = self.imports.get(receiver_name) {
                // Has import: check if it's node_modules (external) or local (project)
                return matches!(resolved, ResolvedImport::Local { .. });
            }

            // No import for receiver - check if it's defined locally
            if self.local_definitions.contains(receiver_name) {
                return true; // Defined in this file - project code
            }

            // No import and not locally defined -> built-in/external
            return false;
        }

        // Case 2: Direct function call - functionName()
        // Check if function has an import
        if let Some(resolved) = self.imports.get(callee_name) {
            // Has import: check if it's node_modules (external) or local (project)
            return matches!(resolved, ResolvedImport::Local { .. });
        }

        // No import - check if defined locally in this file
        if self.local_definitions.contains(callee_name) {
            return true; // Defined in this file - project code
        }

        // No import and not locally defined -> built-in/external
        false
    }
}

/// Find and parse tsconfig.json from the workspace
fn find_tsconfig(workspace_root: &Path) -> Option<(Vec<(String, Vec<String>)>, Option<String>)> {
    let tsconfig_path = workspace_root.join("tsconfig.json");
    if !tsconfig_path.exists() {
        return None;
    }

    let content = std::fs::read_to_string(&tsconfig_path).ok()?;
    parse_tsconfig_paths(&content)
}

/// Parse tsconfig.json to extract paths and baseUrl
fn parse_tsconfig_paths(content: &str) -> Option<(Vec<(String, Vec<String>)>, Option<String>)> {
    // Simple JSON parsing - looking for "compilerOptions" section
    let json: serde_json::Value = serde_json::from_str(content).ok()?;
    let compiler_options = json.get("compilerOptions")?;

    // Extract baseUrl
    let base_url = compiler_options
        .get("baseUrl")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Extract paths
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

/// Resolve an import path using tsconfig path mappings
fn resolve_import_path(
    import_path: &str,
    current_file: &Path,
    workspace_root: &Path,
    path_mappings: &[(String, Vec<String>)],
    base_url: Option<&str>,
) -> Option<String> {
    // Check if it's a node_modules import (doesn't start with . or /)
    if !import_path.starts_with('.') && !import_path.starts_with('/') {
        // Try to resolve as path alias first
        for (pattern, targets) in path_mappings {
            if let Some(resolved) =
                resolve_path_alias(import_path, pattern, targets, workspace_root)
            {
                return Some(resolved);
            }
        }

        // If it looks like a package import (no path alias matched), it's external
        if !import_path.starts_with("~") && !import_path.starts_with("@/") {
            return Some(format!("<node_modules>/{}", import_path));
        }

        // Unknown path alias - try to resolve anyway
        return None;
    }

    // Relative import - resolve against current file directory
    let current_dir = current_file.parent()?;
    let resolved = current_dir.join(import_path);

    // Try bare path first (import already includes extension)
    if let Ok(canonical) = resolved.canonicalize() {
        return Some(canonical.to_string_lossy().to_string());
    }

    // Try appending TypeScript/JavaScript extensions (import without extension)
    let path_str = resolved.to_string_lossy();
    for ext in &[".ts", ".tsx", ".js", ".jsx"] {
        let path_with_ext = PathBuf::from(format!("{}{}", path_str, ext));
        if let Ok(canonical) = path_with_ext.canonicalize() {
            return Some(canonical.to_string_lossy().to_string());
        }
    }

    // Try index files
    for ext in &[".ts", ".tsx", ".js", ".jsx"] {
        let index_path = resolved.join(format!("index{}", ext));
        if let Ok(canonical) = index_path.canonicalize() {
            return Some(canonical.to_string_lossy().to_string());
        }
    }

    None
}

/// Resolve a path alias (e.g., "~/api/user" with "~/*" -> "./app/*")
fn resolve_path_alias(
    import_path: &str,
    pattern: &str,
    targets: &[String],
    workspace_root: &Path,
) -> Option<String> {
    // Pattern like "~/*" should match import "~/api/user"
    if pattern.ends_with("/*") {
        let prefix = &pattern[..pattern.len() - 2]; // "~"
        if import_path.starts_with(prefix) && import_path.len() > prefix.len() {
            let suffix = &import_path[prefix.len()..]; // "/api/user"
                                                       // Remove leading "/" from suffix to make it relative
            let suffix = suffix.strip_prefix('/').unwrap_or(suffix);

            // Use first target
            let target = targets.first()?;
            if target.ends_with("/*") {
                let target_prefix = &target[..target.len() - 2]; // "./app"
                                                                 // Remove leading "./" from target_prefix if present
                let target_prefix = target_prefix.strip_prefix("./").unwrap_or(target_prefix);
                let resolved_path = workspace_root.join(target_prefix).join(suffix);

                // Try canonicalizing with the path as-is
                if let Ok(canonical) = resolved_path.canonicalize() {
                    return Some(canonical.to_string_lossy().to_string());
                }

                // Try adding common TypeScript/JavaScript extensions
                // Note: We append extensions, not replace, to handle files like "auth.server.ts"
                let path_str = resolved_path.to_string_lossy();
                for ext in &[".ts", ".tsx", ".js", ".jsx"] {
                    let path_with_ext = format!("{}{}", path_str, ext);
                    let path_with_ext = PathBuf::from(path_with_ext);
                    if let Ok(canonical) = path_with_ext.canonicalize() {
                        return Some(canonical.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    None
}

/// Extract all imports from a TypeScript file and resolve them
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
                    // Handle require() calls
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

/// Extract all locally defined identifiers from a TypeScript file
/// This includes: function declarations, const/let/var declarations, class declarations
fn extract_local_definitions(root: Node, source: &[u8]) -> std::collections::HashSet<String> {
    let mut definitions = std::collections::HashSet::new();

    for index in 0..root.named_child_count() {
        if let Some(child) = root.named_child(index) {
            extract_definitions_from_node(child, source, &mut definitions);
        }
    }

    definitions
}

/// Recursively extract definitions from a node
fn extract_definitions_from_node(
    node: Node,
    source: &[u8],
    definitions: &mut std::collections::HashSet<String>,
) {
    match node.kind() {
        // Function declarations: function foo() {}
        "function_declaration" | "generator_function_declaration" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Some(name) = node_text(name_node, source) {
                    definitions.insert(name);
                }
            }
            // Don't recurse into function body for top-level definitions
            return;
        }

        // Class declarations: class Foo {}
        "class_declaration" | "class" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Some(name) = node_text(name_node, source) {
                    definitions.insert(name);
                }
            }
            return;
        }

        // Variable declarations: const foo = ..., let bar = ..., var baz = ...
        "variable_declaration" | "lexical_declaration" => {
            extract_variable_names(node, source, definitions);
            return;
        }

        // Export declarations - extract the declaration inside
        "export_statement" | "export_declaration" => {
            // Check for export of declarations
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

        // Type/interface declarations
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

    // Recurse into children for non-handled nodes
    for index in 0..node.named_child_count() {
        if let Some(child) = node.named_child(index) {
            extract_definitions_from_node(child, source, definitions);
        }
    }
}

/// Extract variable names from a variable declaration node
fn extract_variable_names(
    node: Node,
    source: &[u8],
    definitions: &mut std::collections::HashSet<String>,
) {
    // variable_declaration has declarators as children
    for index in 0..node.named_child_count() {
        if let Some(child) = node.named_child(index) {
            if child.kind() == "variable_declarator" {
                // Get the name of the variable
                if let Some(name_node) = child.child_by_field_name("name") {
                    // Handle destructuring patterns simply by extracting identifiers
                    extract_names_from_pattern(name_node, source, definitions);
                }
            }
        }
    }
}

/// Extract identifier names from a pattern (handles simple names and destructuring)
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
        // For destructuring patterns like const { a, b } = obj
        // we extract the property identifiers
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
                        // const { a: b } = obj - get the value (b)
                        if let Some(value) = child.child_by_field_name("value") {
                            extract_names_from_pattern(value, source, definitions);
                        }
                    }
                }
            }
        }
        // For array patterns like const [a, b] = arr
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

/// Extract named imports: import { x, y } from "path"
fn extract_import_declaration(
    node: Node,
    source: &[u8],
    current_file: &Path,
    workspace_root: &Path,
    path_mappings: &[(String, Vec<String>)],
    base_url: Option<&str>,
    imports: &mut HashMap<String, ResolvedImport>,
) {
    // Get the import clause (specifiers) and source
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

    // Clean up the source path (remove surrounding quotes if present)
    let source_path = source_path
        .trim_start_matches('"')
        .trim_end_matches('"')
        .trim_start_matches('\'')
        .trim_end_matches('\'')
        .to_string();

    // Resolve the import path
    let resolved = resolve_import_path(
        &source_path,
        current_file,
        workspace_root,
        path_mappings,
        base_url,
    );

    let import_kind = if source_path.starts_with(".") {
        // Relative import
        if let Some(resolved_path) = resolved {
            ResolvedImport::Local {
                path: resolved_path,
            }
        } else {
            ResolvedImport::Unknown
        }
    } else if source_path.starts_with("~") || source_path.starts_with("@/") {
        // Path alias
        if let Some(resolved_path) = resolved {
            ResolvedImport::Local {
                path: resolved_path,
            }
        } else {
            ResolvedImport::Unknown
        }
    } else {
        // External package
        ResolvedImport::External {
            package: source_path
                .split('/')
                .next()
                .unwrap_or(&source_path)
                .to_string(),
        }
    };

    // Extract imported names
    if let Some(clause) = import_clause {
        extract_import_specifiers(clause, source, &import_kind, imports);
    }
}

/// Extract specifiers from import clause
fn extract_import_specifiers(
    clause: Node,
    source: &[u8],
    import_kind: &ResolvedImport,
    imports: &mut HashMap<String, ResolvedImport>,
) {
    // Handle different import styles:
    // import { a, b } from "..."
    // import * as ns from "..."
    // import defaultImport from "..."

    for index in 0..clause.named_child_count() {
        if let Some(child) = clause.named_child(index) {
            match child.kind() {
                "named_imports" => {
                    // import { a, b } from "..."
                    // named_imports contains import_specifier children
                    for i in 0..child.named_child_count() {
                        if let Some(spec) = child.named_child(i) {
                            if spec.kind() == "import_specifier" {
                                extract_single_import_specifier(spec, source, import_kind, imports);
                            }
                        }
                    }
                }
                "import_specifier" => {
                    // Direct import_specifier (shouldn't happen with named_imports wrapper)
                    extract_single_import_specifier(child, source, import_kind, imports);
                }
                "identifier" => {
                    // Default import: import MyDefault from "..."
                    if let Some(name) = node_text(child, source) {
                        imports.insert(name, import_kind.clone());
                    }
                }
                "namespace_import" => {
                    // import * as name from "..."
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

/// Extract a single import specifier
fn extract_single_import_specifier(
    spec: Node,
    source: &[u8],
    import_kind: &ResolvedImport,
    imports: &mut HashMap<String, ResolvedImport>,
) {
    // import { original as alias }
    // The first identifier is the local name (what we use in code)
    // The second identifier (if present) is the imported name
    let mut local_name = None;

    for i in 0..spec.named_child_count() {
        if let Some(spec_child) = spec.named_child(i) {
            match spec_child.kind() {
                "identifier" => {
                    if local_name.is_none() {
                        local_name = node_text(spec_child, source);
                    }
                    // Second identifier would be the original imported name,
                    // but we only care about the local name for tracking usage
                }
                _ => {}
            }
        }
    }

    let name = local_name.unwrap_or_default();
    if !name.is_empty() {
        imports.insert(name, import_kind.clone());
    }
}

/// Extract require() calls
fn extract_require_call(
    node: Node,
    source: &[u8],
    current_file: &Path,
    workspace_root: &Path,
    path_mappings: &[(String, Vec<String>)],
    base_url: Option<&str>,
    imports: &mut HashMap<String, ResolvedImport>,
) {
    // Look for: const x = require("path")
    // This is simplified - full implementation would need to track variable assignments

    // For now, just check if this expression contains a require call
    // and if there's a variable declaration
    let text = node_text(node, source).unwrap_or_default();

    if text.contains("require(") {
        // Try to extract the require path and variable name
        // This is a simplified regex-like approach
        if let Some(require_idx) = text.find("require(") {
            let after_require = &text[require_idx + 8..];
            if let Some(end_quote) = after_require.find('"').or_else(|| after_require.find('\'')) {
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

                    // Try to find the variable name before require
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

    fn find_callees(
        &self,
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

        // Find workspace root and load tsconfig
        let workspace_root = find_workspace_root(path);
        let (path_mappings, base_url) = find_tsconfig(&workspace_root).unwrap_or_default();

        // Extract imports from the file
        let imports = extract_typescript_imports(
            tree.root_node(),
            source.as_bytes(),
            path,
            &workspace_root,
            &path_mappings,
            base_url.as_deref(),
        );

        // Extract local definitions from the file
        let local_definitions = extract_local_definitions(tree.root_node(), source.as_bytes());

        // Create file context with imports and local definitions
        let file_context = TypeScriptFileContext::new(path_mappings, base_url, local_definitions);

        // Add imports to the context
        let file_context = TypeScriptFileContext {
            imports,
            ..file_context
        };

        // Find the target function/method and extract all its callees
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

        // Convert to absolute path
        for callee in &mut callees {
            if !callee.path.starts_with('/') {
                callee.path = path.to_string_lossy().replace('\\', "/");
            }
        }

        callees
    }

    fn supports_framework(&self, framework: Option<&str>) -> bool {
        match framework {
            None => true,
            Some("react-router") => true,
            Some(_) => false,
        }
    }
}

/// Find workspace root by looking for package.json or tsconfig.json
fn find_workspace_root(start_path: &Path) -> PathBuf {
    let mut current = start_path.parent();
    while let Some(dir) = current {
        if dir.join("package.json").exists() || dir.join("tsconfig.json").exists() {
            return dir.to_path_buf();
        }
        current = dir.parent();
    }
    // Fallback to the file's directory
    start_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
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

struct CalleeContext<'a> {
    target_symbol: &'a str,
    current_file: &'a Path,
    public_language: Option<&'a str>,
    file_context: Option<TypeScriptFileContext>,
}

#[derive(Clone)]
struct FunctionContextForCallee {
    name: String,
    depth: usize,
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
                line_end: (node.end_position().row + 1) as u32,
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
        line_end: (node.end_position().row + 1) as u32,
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

    // Normalize import source: remove quotes and handle various formats
    let import_source = import_source.trim().trim_matches(&['\'', '"'][..]);

    // Check if it's a node_modules import (should not be resolved to local files)
    if !import_source.starts_with('.') && !import_source.starts_with("~/") {
        // This is a package import like "react-router" or "next-auth/react"
        // Skip resolution - these are not local files
        return false;
    }

    // Resolve ~ alias (common in React Router 7/Vite projects)
    // ~ typically maps to the source directory (app/, src/, etc.)
    if import_source.starts_with("~/") {
        let remainder = &import_source[2..]; // Strip the ~/

        // If import already has a file extension, don't add another one
        let has_extension = remainder.ends_with(".ts")
            || remainder.ends_with(".tsx")
            || remainder.ends_with(".js")
            || remainder.ends_with(".jsx");

        let candidates_dirs = ["app", "src"];
        for candidate_dir in candidates_dirs {
            let candidate_path = workspace_root.join(candidate_dir).join(remainder);

            if has_extension {
                // If it already has extension, just check if it exists
                if candidate_path.exists()
                    && normalize_path(&candidate_path) == normalize_path(target_path)
                {
                    return true;
                }
            } else {
                // Try without extension first, then with extensions and index variants
                if candidate_path.exists()
                    || candidate_path.with_extension("ts").exists()
                    || candidate_path.with_extension("tsx").exists()
                    || candidate_path.join("index.ts").exists()
                    || candidate_path.join("index.tsx").exists()
                {
                    return import_matches_candidates(
                        &normalize_path(&candidate_path),
                        target_path,
                    );
                }
            }
        }
        // Fallback: treat ~/ as relative to workspace root
        let fallback_path = workspace_root.join(remainder);
        return import_matches_candidates(&normalize_path(&fallback_path), target_path);
    }

    // Handle relative imports (./ and ../)
    let import_path = if import_source.starts_with('.') {
        normalize_path(&base_dir.join(import_source))
    } else {
        // Absolute path from workspace root
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

/// Collects all function/method calls within a target function.
fn collect_callees(
    node: Node,
    source: &[u8],
    current_function: Option<FunctionContextForCallee>,
    ctx: &CalleeContext,
    callees: &mut Vec<CalleeDefinition>,
) {
    // Check if this node is the target function/method we want to analyze
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
        let name = node
            .child_by_field_name("name")
            .and_then(|n| node_text(n, source))
            .unwrap_or_default();
        Some(FunctionContextForCallee { name, depth: 0 })
    } else {
        current_function.clone()
    };

    // If we're inside the target function, look for call expressions
    if is_target_function || current_function.is_some() {
        if node.kind() == "call_expression" {
            if let Some(callee) = extract_callee(node, source, ctx) {
                callees.push(callee);
            }
        }
    }

    // Recurse into children
    for index in 0..node.named_child_count() {
        if let Some(child) = node.named_child(index) {
            collect_callees(child, source, next_function.clone(), ctx, callees);
        }
    }
}

/// Extracts a callee definition from a call_expression node.
fn extract_callee(node: Node, source: &[u8], ctx: &CalleeContext) -> Option<CalleeDefinition> {
    if node.kind() != "call_expression" {
        return None;
    }

    let callee = node.child_by_field_name("function")?;

    let (callee_name, receiver_type, root_object) = match callee.kind() {
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

            // Find the root object of the member expression chain
            // e.g., for "url.searchParams.get", root is "url"
            let root = find_root_object(callee, source);

            (name, object, root)
        }
        _ => return None,
    };

    // Filter out external library calls using imports + local definitions
    if let Some(ref file_ctx) = ctx.file_context {
        if !file_ctx.is_callee_from_project(receiver_type.as_deref(), &callee_name) {
            return None;
        }
    } else {
        // Without file context, we can't reliably determine - skip filtering
        return None;
    }

    // Get end position for the call expression (to know where it ends)
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

/// Find the root object of a member expression chain
/// e.g., for "url.searchParams.get", returns "url"
fn find_root_object(member_expr: Node, source: &[u8]) -> Option<String> {
    let mut current = member_expr;

    while current.kind() == "member_expression" {
        if let Some(object) = current.child_by_field_name("object") {
            if object.kind() != "member_expression" {
                // This is the root object
                return node_text(object, source);
            }
            current = object;
        } else {
            break;
        }
    }

    None
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
