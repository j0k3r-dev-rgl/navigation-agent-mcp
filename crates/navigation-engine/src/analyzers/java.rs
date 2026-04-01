use std::collections::HashMap;
use std::path::Path;

use tree_sitter::{Node, Parser};

use super::language_analyzer::LanguageAnalyzer;
use super::types::{
    infer_public_language, normalize_public_endpoint_kind, normalize_public_symbol_kind,
    AnalyzerLanguage, CalleeDefinition, CallerDefinition, CallerTarget, EndpointDefinition,
    FindCalleesQuery, FindCallersQuery, FindEndpointsQuery, FindSymbolQuery, SymbolDefinition,
};

/// Context information about a Java file used for filtering callees
#[derive(Clone)]
struct JavaFileContext {
    /// Package declaration of the file (e.g., "com.sistemasias.ar.modules.titular")
    package_name: String,
    /// Base package prefix for the project (detected from package name)
    project_prefix: String,
    /// Map of simple name -> fully qualified name for imports
    /// e.g., "List" -> "java.util.List", "EditTitular" -> "com.sistemasias.ar.modules.titular.application.ports.input.EditTitular"
    imports: HashMap<String, String>,
    /// Map of field name -> type name for class fields
    /// e.g., "editTitularPort" -> "EditTitular"
    class_fields: HashMap<String, String>,
}

impl JavaFileContext {
    fn new(package_name: &str) -> Self {
        // Detect project prefix from package (first 3 segments typically)
        // e.g., "com.sistemasias.ar.modules..." -> "com.sistemasias.ar"
        let project_prefix = package_name
            .split('.')
            .take(3)
            .collect::<Vec<_>>()
            .join(".");

        Self {
            package_name: package_name.to_string(),
            project_prefix,
            imports: HashMap::new(),
            class_fields: HashMap::new(),
        }
    }

    /// Check if a type name belongs to the project (not an external library)
    fn is_project_type(&self, type_name: &str) -> bool {
        // Remove generic parameters if present (e.g., "List<String>" -> "List")
        let base_type = type_name.split('<').next().unwrap_or(type_name).trim();

        // Check if it's in imports
        if let Some(fully_qualified) = self.imports.get(base_type) {
            // If the fully qualified name starts with the project prefix, it's a project type
            return fully_qualified.starts_with(&self.project_prefix);
        }

        // If not in explicit imports, check if it's in the same package
        // (types in the same package don't need import)
        if base_type
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
        {
            // It's a class name (starts with uppercase), likely in the same package
            return true;
        }

        // Unknown type, assume external to be safe
        false
    }

    /// Check if a method call is on a project type
    fn is_callee_from_project(&self, receiver_name: Option<&str>) -> bool {
        let Some(receiver) = receiver_name else {
            // No receiver - could be:
            // 1. Static method import: import static java.util.Map.*; -> of()
            // 2. Method in same class or superclass
            // 3. Method in enclosing class
            // Without full type resolution, we can't distinguish reliably.
            // Conservatively: assume it might be project code.
            return true;
        };

        // Check if receiver is a field
        if let Some(field_type) = self.class_fields.get(receiver) {
            return self.is_project_type(field_type);
        }

        // Check receiver naming patterns for common external types
        // These are heuristics based on common naming conventions
        let lower = receiver.to_lowercase();
        if lower.contains("request") && !lower.contains("titular") && !lower.contains("member") {
            // HttpServletRequest, etc. - external
            return false;
        }
        if lower.contains("response") {
            // HttpServletResponse, etc. - external
            return false;
        }
        if lower.starts_with("jwt") || (lower.contains("service") && lower.contains("jwt")) {
            // JwtService - external
            return false;
        }
        if lower == "map"
            || lower == "list"
            || lower == "set"
            || lower == "collections"
            || lower == "arrays"
        {
            // java.util.* utility classes
            return false;
        }
        if lower.starts_with("string") || lower == "system" || lower == "objects" || lower == "math"
        {
            // java.lang.* classes
            return false;
        }
        // Spring and Jakarta classes
        if lower == "responseentity"
            || lower == "httpstatus"
            || lower == "httpservletrequest"
            || lower == "httpservletresponse"
        {
            return false;
        }

        // Unknown receiver, assume it might be project code
        true
    }
}

/// Filter for reducing noise in callee extraction.
///
/// Evaluates each callee against rules in priority order:
/// 1. Constructor preservation (always include object_creation_expression)
/// 2. Object method filtering (toString, equals, hashCode, etc.)
/// 3. Exception allowlist (Port types, project types)
/// 4. Package-based filtering (java.*, jakarta.*, javax.*, org.springframework.*)
/// 5. Model getter/setter filtering
/// 6. Default include
///
/// For Iteration 1: Basic filtering without builder chain tracking
struct CalleeFilter {
    /// Reference to file context for type resolution
    file_context: JavaFileContext,
}

impl CalleeFilter {
    /// Create a new filter with file context
    fn new(file_context: JavaFileContext) -> Self {
        Self { file_context }
    }

    /// Determine if a callee should be included in results.
    ///
    /// Priority order (from spec):
    /// 1. Constructor Preservation
    /// 2. Object Method Filtering
    /// 3. Exception Allowlist (Ports, project types)
    /// 4. Package-Based Filtering
    /// 5. Model Getter/Setter Filtering
    /// 6. Default: Include
    ///
    /// # Arguments
    /// * `callee_name` - The method name being called
    /// * `receiver_name` - Optional name of the receiver (variable/field name)
    /// * `receiver_type` - Optional resolved type of the receiver
    /// * `is_constructor` - Whether this is an object_creation_expression
    fn should_include(
        &self,
        callee_name: &str,
        receiver_name: Option<&str>,
        receiver_type: Option<&str>,
        is_constructor: bool,
    ) -> bool {
        // === PRIORITY 1: Constructor Preservation ===
        if is_constructor {
            return true;
        }

        // === PRIORITY 2: Object Method Filtering ===
        if Self::is_object_method(callee_name) {
            return false;
        }

        // === PRIORITY 3: Exception Allowlist ===
        // Port types are always included (business interfaces)
        if let Some(rtype) = receiver_type {
            if rtype.ends_with("Port") || rtype.contains("Port<") {
                return true;
            }
        }

        // Project types are generally included (but check for getters/setters later)
        let is_project_type = if let Some(ref recv) = receiver_name {
            self.file_context.is_callee_from_project(Some(recv))
        } else {
            // No receiver - could be static import or same-class method
            // Conservatively treat as potentially project code
            true
        };

        // === PRIORITY 4: Package-Based Filtering ===
        // Resolve the type to its fully qualified name using imports
        let resolved_type = receiver_type.map(|rt| {
            // If already fully qualified, use as-is
            if rt.contains('.') {
                rt.to_string()
            } else {
                // Look up in imports
                self.file_context
                    .imports
                    .get(rt)
                    .cloned()
                    .unwrap_or_else(|| {
                        // Common java.lang types are implicitly imported
                        // Check if it's a known java.lang type
                        if Self::is_java_lang_type(rt) {
                            format!("java.lang.{}", rt)
                        } else {
                            rt.to_string()
                        }
                    })
            }
        });

        if let Some(ref rtype) = resolved_type {
            // java.*, jakarta.* always filtered
            if rtype.starts_with("java.") || rtype.starts_with("jakarta.") {
                return false;
            }
            // javax.* filtered
            if rtype.starts_with("javax.") {
                return false;
            }
            // org.springframework.* filtered except Ports (already checked above)
            if rtype.starts_with("org.springframework.") {
                return false;
            }
        }

        // === PRIORITY 5: Model Getter/Setter Filtering ===
        // Only filter getters/setters on model types, not on business types
        if Self::is_getter_setter(callee_name) {
            // If it's a project type, check if it's a model type
            if is_project_type {
                if self.is_model_type(receiver_type) {
                    return false;
                }
            }
        }

        // === PRIORITY 6: Noise Receiver Type Filtering ===
        // Filter out calls where receiver_type is clearly not a type:
        // - Builder chain expressions: "Type.builder()..." or "Type.builder().method()..."
        // - Variable names (lowercase, no package dots): "memberSaved", "rootData"
        if Self::is_noise_receiver_type(receiver_type) {
            return false;
        }

        // === DEFAULT: Include ===
        true
    }

    /// Check if method name is a standard Object method
    fn is_object_method(name: &str) -> bool {
        matches!(
            name,
            "toString"
                | "equals"
                | "hashCode"
                | "getClass"
                | "clone"
                | "notify"
                | "notifyAll"
                | "wait"
        )
    }

    /// Check if method name matches getter or setter pattern
    fn is_getter_setter(name: &str) -> bool {
        (name.starts_with("get")
            && name.len() > 3
            && name.chars().nth(3).map_or(false, |c| c.is_uppercase()))
            || (name.starts_with("is")
                && name.len() > 2
                && name.chars().nth(2).map_or(false, |c| c.is_uppercase()))
            || (name.starts_with("set")
                && name.len() > 3
                && name.chars().nth(3).map_or(false, |c| c.is_uppercase()))
    }

    /// Check if a type name is from java.lang (implicitly imported)
    fn is_java_lang_type(name: &str) -> bool {
        matches!(
            name,
            "String"
                | "Object"
                | "Integer"
                | "Long"
                | "Double"
                | "Float"
                | "Boolean"
                | "Byte"
                | "Short"
                | "Character"
                | "System"
                | "Math"
                | "StringBuilder"
                | "StringBuffer"
                | "Exception"
                | "RuntimeException"
                | "Throwable"
                | "Class"
                | "Enum"
        )
    }

    /// Check if a type is a model/persistence type based on naming patterns
    fn is_model_type(&self, receiver_type: Option<&str>) -> bool {
        match receiver_type {
            Some(rtype) => {
                // Remove generic parameters if present
                let base = rtype.split('<').next().unwrap_or(rtype);
                // Get the simple name (last segment)
                let name = base.split('.').last().unwrap_or(base);

                // Name-based heuristics for model types
                name.ends_with("PersistenceModel")
                    || name.ends_with("Entity")
                    || (name.ends_with("Model") && !name.ends_with("ViewModel"))
                    || name.ends_with("DTO")
                    || name.ends_with("Request")
                    || name.ends_with("Response")
            }
            None => false,
        }
    }

    /// Check if receiver_type is "garbage" - not a proper type.
    /// This catches:
    /// - Builder chain expressions like "TitularPersistenceModel.builder()..."
    /// - Variable names that aren't types (lowercase start, no dots)
    /// - Other non-type expressions
    fn is_noise_receiver_type(receiver_type: Option<&str>) -> bool {
        match receiver_type {
            Some(rtype) => {
                // If it contains builder chain pattern, it's noise
                if rtype.contains(".builder(") || rtype.contains(".build(") {
                    return true;
                }
                // If it looks like a builder chain continuation (multiple dots with method calls)
                if rtype.contains(").") && rtype.chars().any(|c| c == '(' || c == ')') {
                    return true;
                }
                // If it's a simple name (no dots) and starts with lowercase, it's likely a variable
                if !rtype.contains('.') && rtype.chars().next().map_or(false, |c| c.is_lowercase())
                {
                    return true;
                }
                false
            }
            None => false,
        }
    }
}

pub struct JavaAnalyzer;

impl LanguageAnalyzer for JavaAnalyzer {
    fn language(&self) -> AnalyzerLanguage {
        AnalyzerLanguage::Java
    }

    fn supported_extensions(&self) -> &'static [&'static str] {
        &[".java"]
    }

    fn find_symbols(
        &self,
        path: &Path,
        source: &str,
        _query: &FindSymbolQuery,
    ) -> Vec<SymbolDefinition> {
        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_java::LANGUAGE.into())
            .is_err()
        {
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
        query: &FindEndpointsQuery,
    ) -> Vec<EndpointDefinition> {
        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_java::LANGUAGE.into())
            .is_err()
        {
            return Vec::new();
        }

        let Some(tree) = parser.parse(source, None) else {
            return Vec::new();
        };

        let public_language = infer_public_language(path);
        let mut endpoints = Vec::new();
        collect_endpoints(
            tree.root_node(),
            source.as_bytes(),
            public_language.as_deref(),
            &mut endpoints,
        );

        // Filter by kind
        let filtered: Vec<EndpointDefinition> = endpoints
            .into_iter()
            .filter(|e| {
                if query.kind == "any" {
                    true
                } else {
                    e.kind == query.kind
                }
            })
            .take(query.limit)
            .collect();

        filtered
    }

    fn find_callers(
        &self,
        _workspace_root: &Path,
        path: &Path,
        source: &str,
        query: &FindCallersQuery,
    ) -> Vec<CallerDefinition> {
        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_java::LANGUAGE.into())
            .is_err()
        {
            return Vec::new();
        }

        let Some(tree) = parser.parse(source, None) else {
            return Vec::new();
        };

        let public_language = infer_public_language(path);
        let mut callers = Vec::new();
        collect_java_callers(
            tree.root_node(),
            source.as_bytes(),
            public_language.as_deref(),
            query,
            None,
            None,
            &mut callers,
        );
        callers
    }

    fn find_callees(
        &self,
        path: &Path,
        source: &str,
        query: &FindCalleesQuery,
    ) -> Vec<CalleeDefinition> {
        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_java::LANGUAGE.into())
            .is_err()
        {
            return Vec::new();
        }

        let Some(tree) = parser.parse(source, None) else {
            return Vec::new();
        };

        // First pass: extract file context (package, imports, class fields)
        let file_ctx = extract_file_context(tree.root_node(), source.as_bytes());

        let public_language = infer_public_language(path);
        let mut callees = Vec::new();

        // Create filter from file context
        let callee_filter = Some(CalleeFilter::new(file_ctx.clone()));

        let mut ctx = JavaCalleeContext {
            target_symbol: &query.target_symbol,
            current_file: path,
            public_language: public_language.as_deref(),
            file_context: Some(file_ctx),
            callee_filter,
            active_builder_chains: HashMap::new(),
        };

        collect_java_callees(
            tree.root_node(),
            source.as_bytes(),
            None,
            &mut ctx,
            &mut callees,
        );

        callees
    }

    fn supports_framework(&self, framework: Option<&str>) -> bool {
        match framework {
            None => true,
            Some("spring") => true,
            Some(_) => false,
        }
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
        "class_declaration" => (node.child_by_field_name("name")?, "class_declaration"),
        "interface_declaration" => (node.child_by_field_name("name")?, "interface_declaration"),
        "enum_declaration" => (node.child_by_field_name("name")?, "enum_declaration"),
        "annotation_type_declaration" => (node.child_by_field_name("name")?, "annotation_type"),
        "record_declaration" => (node.child_by_field_name("name")?, "record"),
        "method_declaration" => (node.child_by_field_name("name")?, "method_declaration"),
        "constructor_declaration" => (node.child_by_field_name("name")?, "constructor_declaration"),
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

/// Collects Spring REST and GraphQL endpoints from Java class declarations.
fn collect_endpoints(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
    endpoints: &mut Vec<EndpointDefinition>,
) {
    // Only process class_declaration nodes
    if node.kind() != "class_declaration" {
        for index in 0..node.named_child_count() {
            if let Some(child) = node.named_child(index) {
                collect_endpoints(child, source, public_language, endpoints);
            }
        }
        return;
    }

    // Get class modifiers - it's a child node, not a field
    let modifiers = find_modifiers_child(&node);

    // Determine controller type (REST or GraphQL)
    let (is_rest_controller, is_graphql_controller) = match &modifiers {
        Some(m) => check_controller_type(m, source),
        None => (false, false),
    };

    if !is_rest_controller && !is_graphql_controller {
        return;
    }

    // Extract class-level @RequestMapping path (base path)
    let base_path = modifiers
        .as_ref()
        .and_then(|m| extract_request_mapping_path(m, source));

    // Find class_body and traverse method declarations
    let class_body = match node.child_by_field_name("body") {
        Some(b) => b,
        None => return,
    };

    // Process all method declarations in the class body
    for index in 0..class_body.named_child_count() {
        if let Some(child) = class_body.named_child(index) {
            if child.kind() == "method_declaration" {
                if is_rest_controller {
                    extract_rest_endpoints(
                        &child,
                        source,
                        public_language,
                        base_path.as_deref(),
                        endpoints,
                    );
                }
                if is_graphql_controller {
                    extract_graphql_endpoints(&child, source, public_language, endpoints);
                }
            }
        }
    }
}

/// Finds the modifiers child node in a class_declaration or method_declaration.
fn find_modifiers_child<'a>(node: &Node<'a>) -> Option<Node<'a>> {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "modifiers" {
                return Some(child);
            }
        }
    }
    None
}

/// Checks if a class is a REST controller (@RestController) or GraphQL controller (@Controller).
fn check_controller_type(modifiers: &Node, source: &[u8]) -> (bool, bool) {
    let mut is_rest_controller = false;
    let mut is_graphql_controller = false;

    for index in 0..modifiers.named_child_count() {
        if let Some(child) = modifiers.named_child(index) {
            let annotation_name = match child.kind() {
                "marker_annotation" => extract_marker_annotation_name(&child, source),
                "annotation" => extract_annotation_name(&child, source),
                _ => None,
            };

            if let Some(name) = annotation_name {
                match name.as_str() {
                    "RestController" => is_rest_controller = true,
                    "Controller" => is_graphql_controller = true,
                    _ => {}
                }
            }
        }
    }

    (is_rest_controller, is_graphql_controller)
}

/// Extracts the path from a class-level @RequestMapping annotation.
fn extract_request_mapping_path(modifiers: &Node, source: &[u8]) -> Option<String> {
    for index in 0..modifiers.named_child_count() {
        if let Some(child) = modifiers.named_child(index) {
            if child.kind() == "annotation" {
                let name = extract_annotation_name(&child, source);
                if name.as_deref() == Some("RequestMapping") {
                    return extract_annotation_path(&child, source);
                }
            }
        }
    }
    None
}

/// Extracts the annotation name from a marker_annotation node.
fn extract_marker_annotation_name(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| node_text(n, source))
}

/// Extracts the annotation name from an annotation node.
fn extract_annotation_name(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| node_text(n, source))
}

/// Extracts the path string from an annotation's argument list.
fn extract_annotation_path(node: &Node, source: &[u8]) -> Option<String> {
    let args = node.child_by_field_name("arguments")?;

    // Look for string_literal in the arguments
    for index in 0..args.named_child_count() {
        if let Some(arg) = args.named_child(index) {
            if arg.kind() == "string_literal" {
                // string_literal contains string_fragment
                for i in 0..arg.named_child_count() {
                    if let Some(fragment) = arg.named_child(i) {
                        if fragment.kind() == "string_fragment" {
                            return node_text(fragment, source);
                        }
                    }
                }
            }
        }
    }
    None
}

/// Extracts REST endpoints from a method declaration.
fn extract_rest_endpoints(
    method_node: &Node,
    source: &[u8],
    public_language: Option<&str>,
    base_path: Option<&str>,
    endpoints: &mut Vec<EndpointDefinition>,
) {
    let modifiers = match find_modifiers_child(method_node) {
        Some(m) => m,
        None => return,
    };

    // Check for REST mapping annotations
    let rest_annotations = [
        "GetMapping",
        "PostMapping",
        "PutMapping",
        "DeleteMapping",
        "PatchMapping",
        "RequestMapping",
    ];

    for index in 0..modifiers.named_child_count() {
        if let Some(child) = modifiers.named_child(index) {
            let (annotation_name, path) = match child.kind() {
                "marker_annotation" => {
                    let name = extract_marker_annotation_name(&child, source);
                    (name, None)
                }
                "annotation" => {
                    let name = extract_annotation_name(&child, source);
                    let path = extract_annotation_path(&child, source);
                    (name, path)
                }
                _ => continue,
            };

            if let Some(name) = annotation_name {
                if rest_annotations.contains(&name.as_str()) {
                    // Get method name
                    let method_name = method_node
                        .child_by_field_name("name")
                        .and_then(|n| node_text(n, source))
                        .unwrap_or_default();

                    // Combine base path with method path
                    let full_path = combine_paths(base_path, path.as_deref());

                    endpoints.push(EndpointDefinition {
                        name: method_name,
                        kind: normalize_public_endpoint_kind(&name),
                        path: full_path,
                        file: String::new(),
                        line: (method_node.start_position().row + 1) as u32,
                        language: public_language.map(str::to_string),
                        framework: Some("spring".to_string()),
                    });
                }
            }
        }
    }
}

/// Extracts GraphQL endpoints from a method declaration.
fn extract_graphql_endpoints(
    method_node: &Node,
    source: &[u8],
    public_language: Option<&str>,
    endpoints: &mut Vec<EndpointDefinition>,
) {
    let modifiers = match find_modifiers_child(method_node) {
        Some(m) => m,
        None => return,
    };

    // GraphQL mapping annotations
    let graphql_annotations = ["QueryMapping", "MutationMapping", "SubscriptionMapping"];

    for index in 0..modifiers.named_child_count() {
        if let Some(child) = modifiers.named_child(index) {
            let annotation_name = match child.kind() {
                "marker_annotation" => extract_marker_annotation_name(&child, source),
                "annotation" => extract_annotation_name(&child, source),
                _ => continue,
            };

            if let Some(name) = annotation_name {
                if graphql_annotations.contains(&name.as_str()) {
                    // Get method name (field name in GraphQL schema)
                    let method_name = method_node
                        .child_by_field_name("name")
                        .and_then(|n| node_text(n, source))
                        .unwrap_or_default();

                    // GraphQL field name is the method name (no path combination)
                    let graphql_field = method_name.clone();

                    endpoints.push(EndpointDefinition {
                        name: graphql_field,
                        kind: normalize_public_endpoint_kind(&name),
                        path: None, // GraphQL has no path, field name is the identifier
                        file: String::new(),
                        line: (method_node.start_position().row + 1) as u32,
                        language: public_language.map(str::to_string),
                        framework: Some("spring".to_string()),
                    });
                }
            }
        }
    }
}

/// Combines base path and method path.
/// Examples:
/// - ("/titulares", "/{id}") → "/titulares/{id}"
/// - ("/titulares", None) → "/titulares"
/// - (None, "/items") → "/items"
fn combine_paths(base: Option<&str>, method: Option<&str>) -> Option<String> {
    match (base, method) {
        (Some(base), Some(method)) => {
            // Ensure proper path concatenation
            let base = base.trim_start_matches('/');
            let method = method.trim_start_matches('/');
            if method.is_empty() {
                Some(format!("/{}", base))
            } else {
                Some(format!("/{}/{}", base, method))
            }
        }
        (Some(base), None) => Some(format!("/{}", base.trim_start_matches('/'))),
        (None, Some(method)) => Some(format!("/{}", method.trim_start_matches('/'))),
        (None, None) => None,
    }
}

fn collect_java_callers(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
    query: &FindCallersQuery,
    current_class: Option<String>,
    current_method: Option<(String, Vec<String>)>,
    callers: &mut Vec<CallerDefinition>,
) {
    let next_class = if node.kind() == "class_declaration" {
        node.child_by_field_name("name")
            .and_then(|item| node_text(item, source))
            .or(current_class.clone())
    } else {
        current_class.clone()
    };

    let next_method = if matches!(
        node.kind(),
        "method_declaration" | "constructor_declaration"
    ) {
        let name = node
            .child_by_field_name("name")
            .and_then(|item| node_text(item, source));
        name.map(|method_name| {
            let caller_display = next_class
                .as_ref()
                .map(|class_name| format!("{}#{}", class_name, method_name))
                .unwrap_or_else(|| method_name.clone());
            let reasons = extract_probable_entry_point_reasons(node, source);
            (caller_display, reasons)
        })
    } else {
        current_method.clone()
    };

    if node.kind() == "method_invocation" {
        if let Some(caller) = extract_java_call(node, source, public_language, query, &next_method)
        {
            callers.push(caller);
        }
    }

    for index in 0..node.named_child_count() {
        if let Some(child) = node.named_child(index) {
            collect_java_callers(
                child,
                source,
                public_language,
                query,
                next_class.clone(),
                next_method.clone(),
                callers,
            );
        }
    }
}

fn extract_java_call(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
    query: &FindCallersQuery,
    current_method: &Option<(String, Vec<String>)>,
) -> Option<CallerDefinition> {
    let name = node
        .child_by_field_name("name")
        .and_then(|item| node_text(item, source))?;
    if name != query.target_symbol {
        return None;
    }

    let (caller_display, reasons) = current_method.as_ref()?.clone();
    let receiver_type = node
        .child_by_field_name("object")
        .and_then(|item| node_text(item, source));

    Some(CallerDefinition {
        path: String::new(),
        line: (node.start_position().row + 1) as u32,
        column: Some((node.start_position().column + 1) as u32),
        caller: caller_display.clone(),
        caller_symbol: Some(caller_display),
        relation: "calls".to_string(),
        language: public_language.map(str::to_string),
        snippet: node_text(node, source),
        receiver_type: receiver_type.clone(),
        calls: CallerTarget {
            path: query.target_path.to_string_lossy().replace('\\', "/"),
            symbol: query.target_symbol.clone(),
        },
        probable_entry_point_reasons: reasons,
    })
}

fn extract_probable_entry_point_reasons(node: Node, source: &[u8]) -> Vec<String> {
    let Some(modifiers) = find_modifiers_child(&node) else {
        return Vec::new();
    };

    let mut reasons = Vec::new();
    for index in 0..modifiers.named_child_count() {
        let Some(child) = modifiers.named_child(index) else {
            continue;
        };
        let annotation_name = match child.kind() {
            "marker_annotation" => extract_marker_annotation_name(&child, source),
            "annotation" => extract_annotation_name(&child, source),
            _ => None,
        };

        match annotation_name.as_deref() {
            Some("GetMapping")
            | Some("PostMapping")
            | Some("PutMapping")
            | Some("DeleteMapping")
            | Some("PatchMapping")
            | Some("RequestMapping") => reasons.push("public controller method".to_string()),
            Some("QueryMapping") | Some("MutationMapping") | Some("SubscriptionMapping") => {
                reasons.push("public graphql method".to_string())
            }
            _ => {}
        }
    }
    reasons
}

struct JavaCalleeContext<'a> {
    target_symbol: &'a str,
    current_file: &'a Path,
    public_language: Option<&'a str>,
    file_context: Option<JavaFileContext>,
    /// Optional filter for reducing noise in callee extraction
    callee_filter: Option<CalleeFilter>,
    /// Tracks active builder chains: receiver variable name -> builder type name
    /// Used to filter chained builder method calls (builder(), field(), build())
    active_builder_chains: HashMap<String, String>,
}

#[derive(Clone)]
struct JavaFunctionContext {
    name: String,
    class_name: Option<String>,
}

fn collect_java_callees(
    node: Node,
    source: &[u8],
    current_function: Option<JavaFunctionContext>,
    ctx: &mut JavaCalleeContext,
    callees: &mut Vec<CalleeDefinition>,
) {
    // Check if this node is a method declaration we're looking for
    let node_kind = node.kind();
    let is_target_method = matches!(node_kind, "method_declaration" | "constructor_declaration")
        && node
            .child_by_field_name("name")
            .and_then(|n| java_node_text(n, source))
            .map(|name| {
                let found = name == ctx.target_symbol;
                eprintln!(
                    "DEBUG: checking method '{}' against target '{}' = {}",
                    name, ctx.target_symbol, found
                );
                found
            })
            .unwrap_or(false);

    // Check if this node is a class declaration - update class context
    let next_class_name =
        if node.kind() == "class_declaration" || node.kind() == "interface_declaration" {
            node.child_by_field_name("name")
                .and_then(|n| java_node_text(n, source))
                .or_else(|| {
                    current_function
                        .as_ref()
                        .map(|f| f.class_name.as_deref())
                        .flatten()
                        .map(String::from)
                })
        } else {
            current_function
                .as_ref()
                .map(|f| f.class_name.as_deref())
                .flatten()
                .map(String::from)
        };

    let next_function = if is_target_method || current_function.is_some() {
        let name = node
            .child_by_field_name("name")
            .and_then(|n| java_node_text(n, source))
            .unwrap_or_default();
        Some(JavaFunctionContext {
            name,
            class_name: next_class_name,
        })
    } else {
        current_function.clone()
    };

    // If we're inside the target method, look for method invocations
    if is_target_method || current_function.is_some() {
        if node.kind() == "method_invocation" {
            // Extract receiver name from file context
            let receiver_name = node
                .child_by_field_name("object")
                .and_then(|n| java_node_text(n, source));

            // Extract receiver type from file context if available
            let receiver_type = receiver_name.as_ref().and_then(|recv| {
                ctx.file_context
                    .as_ref()
                    .and_then(|fc| fc.class_fields.get(recv).cloned())
            });

            // Extract callee name for builder chain detection
            let callee_name = node
                .child_by_field_name("name")
                .and_then(|n| java_node_text(n, source));

            // --- Builder Chain Tracking ---
            let mut is_builder_chain_call = false;
            if let (Some(recv), Some(name)) = (receiver_name.as_ref(), callee_name.as_ref()) {
                // Check if receiver is a known builder (chained call)
                if ctx.active_builder_chains.contains_key(recv) {
                    is_builder_chain_call = true;

                    // Check if this is .build() - end of chain
                    if name.as_str() == "build" {
                        ctx.active_builder_chains.remove(recv);
                    }
                }

                // Check if this is .builder() on a model type - start of chain
                if name.as_str() == "builder" {
                    // Check if receiver type is a model type
                    let is_model = receiver_type.as_ref().map_or(false, |rt| {
                        let base = rt.split('<').next().unwrap_or(rt);
                        let type_name = base.split('.').last().unwrap_or(base);
                        type_name.ends_with("Builder")
                            || type_name.ends_with("PersistenceModel")
                            || type_name.ends_with("Entity")
                            || (type_name.ends_with("Model") && !type_name.ends_with("ViewModel"))
                            || type_name.ends_with("DTO")
                    });

                    if is_model {
                        // Generate a unique identifier for this builder chain
                        // Use the receiver variable name or generate one
                        let chain_id = format!("{}_builder_{}", recv, node.start_position().row);
                        ctx.active_builder_chains.insert(chain_id, (*recv).clone());
                    }
                }
            }

            if let Some(callee) = extract_java_callee(node, source, ctx, &current_function) {
                // Skip chained builder calls (but not the initial .builder() or final .build())
                if is_builder_chain_call && callee.callee != "build" {
                    // This is a chained call on a builder - skip it
                } else {
                    // Apply filter if available
                    let should_include = ctx.callee_filter.as_ref().map_or(true, |filter| {
                        filter.should_include(
                            &callee.callee,
                            receiver_name.as_deref(),
                            callee.receiver_type.as_deref().or(receiver_type.as_deref()),
                            false, // is_constructor = false
                        )
                    });

                    if should_include {
                        callees.push(callee);
                    }
                }
            }
        }
        // Also check for constructor calls (new Object())
        if node.kind() == "object_creation_expression" {
            if let Some(callee) = extract_java_callee(node, source, ctx, &current_function) {
                // Constructors are always included (Priority 1)
                callees.push(callee);
            }
        }
    }

    // Recurse into children
    for index in 0..node.named_child_count() {
        if let Some(child) = node.named_child(index) {
            collect_java_callees(child, source, next_function.clone(), ctx, callees);
        }
    }
}

fn extract_java_callee(
    node: Node,
    source: &[u8],
    ctx: &JavaCalleeContext,
    current_function: &Option<JavaFunctionContext>,
) -> Option<CalleeDefinition> {
    let (callee_name, receiver_name) = match node.kind() {
        "method_invocation" => {
            let receiver = node
                .child_by_field_name("object")
                .and_then(|n| java_node_text(n, source));
            let name = node
                .child_by_field_name("name")
                .and_then(|n| java_node_text(n, source))?;
            (name, receiver)
        }
        "object_creation_expression" => {
            let type_node = node.child_by_field_name("type");
            let name = type_node
                .and_then(|n| java_node_text(n, source))
                .unwrap_or_else(|| "constructor".to_string());
            (name, None)
        }
        _ => return None,
    };

    // NOTE: No filtering here - we return ALL callees
    // Filtering will be done in trace_flow using the global project index

    // Get end line for the call
    let end_line = (node.end_position().row + 1) as u32;

    // Create callee symbol (ClassName.methodName or just methodName)
    let callee_symbol = current_function
        .as_ref()
        .and_then(|f| f.class_name.as_ref())
        .map(|class| format!("{}#{}", class, callee_name));

    // Get the actual type of the receiver (not just the field name)
    // This is needed to trace through interfaces
    let receiver_type = if let Some(ref file_ctx) = ctx.file_context {
        if let Some(ref receiver) = receiver_name {
            // Look up the field type in class_fields
            file_ctx
                .class_fields
                .get(receiver)
                .cloned()
                .or(receiver_name)
        } else {
            receiver_name
        }
    } else {
        receiver_name
    };

    Some(CalleeDefinition {
        path: ctx.current_file.to_string_lossy().replace('\\', "/"),
        line: (node.start_position().row + 1) as u32,
        end_line,
        column: Some((node.start_position().column + 1) as u32),
        callee: callee_name,
        callee_symbol,
        receiver_type,
        relation: "calls".to_string(),
        language: ctx.public_language.map(String::from),
        snippet: java_node_text(node, source),
    })
}

fn java_node_text(node: Node, source: &[u8]) -> Option<String> {
    node.utf8_text(source)
        .ok()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

/// Extract file-level context: package, imports, and class fields
fn extract_file_context(root: Node, source: &[u8]) -> JavaFileContext {
    let mut package_name = String::new();
    let mut imports: HashMap<String, String> = HashMap::new();
    let mut class_fields: HashMap<String, String> = HashMap::new();

    // First pass: collect package and imports
    for index in 0..root.named_child_count() {
        if let Some(child) = root.named_child(index) {
            match child.kind() {
                "package_declaration" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        package_name = java_node_text(name_node, source).unwrap_or_default();
                    }
                }
                "import_declaration" => {
                    if let Some((simple_name, full_name)) = extract_import(child, source) {
                        imports.insert(simple_name, full_name);
                    }
                }
                "class_declaration" | "interface_declaration" => {
                    // Collect fields from class body
                    if let Some(body) = child.child_by_field_name("body") {
                        collect_class_fields(body, source, &mut class_fields);
                    }
                }
                _ => {}
            }
        }
    }

    // If no package found, use empty string
    if package_name.is_empty() {
        package_name = "unknown".to_string();
    }

    let mut ctx = JavaFileContext::new(&package_name);
    ctx.imports = imports;
    ctx.class_fields = class_fields;
    ctx
}

/// Extract a single import declaration: returns (simple_name, fully_qualified_name)
fn extract_import(node: Node, source: &[u8]) -> Option<(String, String)> {
    // import_declaration has a child which is the scoped_identifier or identifier
    let name_node = node
        .named_children(&mut node.walk())
        .find(|c| c.kind() == "scoped_identifier" || c.kind() == "identifier")?;

    let full_name = java_node_text(name_node, source)?;

    // Get the simple name (last segment)
    let simple_name = full_name
        .split('.')
        .last()
        .unwrap_or(&full_name)
        .to_string();

    Some((simple_name, full_name))
}

/// Collect class fields from a class body
fn collect_class_fields(body: Node, source: &[u8], fields: &mut HashMap<String, String>) {
    for index in 0..body.named_child_count() {
        if let Some(child) = body.named_child(index) {
            if child.kind() == "field_declaration" {
                extract_field_declaration(child, source, fields);
            }
        }
    }
}

/// Extract field name and type from a field declaration
fn extract_field_declaration(node: Node, source: &[u8], fields: &mut HashMap<String, String>) {
    // field_declaration: type declarator ("," declarator)* ";"
    let type_node = node.named_children(&mut node.walk()).find(|c| {
        c.kind() == "type_identifier"
            || c.kind() == "scoped_type_identifier"
            || c.kind() == "generic_type"
    });

    let type_name = type_node.and_then(|n| java_node_text(n, source));

    // Find all declarators (variable names)
    for index in 0..node.named_child_count() {
        if let Some(child) = node.named_child(index) {
            if child.kind() == "variable_declarator" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    if let Some(field_name) = java_node_text(name_node, source) {
                        let type_str = type_name.clone().unwrap_or_default();
                        fields.insert(field_name, type_str);
                    }
                }
            }
        }
    }
}
