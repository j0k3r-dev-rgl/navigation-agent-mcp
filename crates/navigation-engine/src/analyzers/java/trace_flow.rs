use std::collections::HashMap;
use std::path::Path;

use tree_sitter::{Node, Parser};

use super::super::types::{infer_public_language, CalleeDefinition, FindCalleesQuery};

#[derive(Clone)]
pub(super) struct JavaFileContext {
    package_name: String,
    project_prefix: String,
    pub(super) imports: HashMap<String, String>,
    pub(super) class_fields: HashMap<String, String>,
}

impl JavaFileContext {
    pub(super) fn new(package_name: &str) -> Self {
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

    fn is_project_type(&self, type_name: &str) -> bool {
        let base_type = type_name.split('<').next().unwrap_or(type_name).trim();

        if let Some(fully_qualified) = self.imports.get(base_type) {
            return fully_qualified.starts_with(&self.project_prefix);
        }

        if base_type
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
        {
            return true;
        }

        false
    }

    fn is_callee_from_project(&self, receiver_name: Option<&str>) -> bool {
        let Some(receiver) = receiver_name else {
            return true;
        };

        if let Some(field_type) = self.class_fields.get(receiver) {
            return self.is_project_type(field_type);
        }

        let lower = receiver.to_lowercase();
        if lower.contains("request") && !lower.contains("titular") && !lower.contains("member") {
            return false;
        }
        if lower.contains("response") {
            return false;
        }
        if lower.starts_with("jwt") || (lower.contains("service") && lower.contains("jwt")) {
            return false;
        }
        if lower == "map"
            || lower == "list"
            || lower == "set"
            || lower == "collections"
            || lower == "arrays"
        {
            return false;
        }
        if lower.starts_with("string") || lower == "system" || lower == "objects" || lower == "math"
        {
            return false;
        }
        if lower == "responseentity"
            || lower == "httpstatus"
            || lower == "httpservletrequest"
            || lower == "httpservletresponse"
        {
            return false;
        }

        true
    }
}

pub(super) struct CalleeFilter {
    file_context: JavaFileContext,
}

impl CalleeFilter {
    pub(super) fn new(file_context: JavaFileContext) -> Self {
        Self { file_context }
    }

    pub(super) fn should_include(
        &self,
        callee_name: &str,
        receiver_name: Option<&str>,
        receiver_type: Option<&str>,
        is_constructor: bool,
    ) -> bool {
        if is_constructor {
            return true;
        }

        if Self::is_object_method(callee_name) {
            return false;
        }

        if let Some(rtype) = receiver_type {
            if rtype.ends_with("Port") || rtype.contains("Port<") {
                return true;
            }
        }

        let is_project_type = if let Some(ref recv) = receiver_name {
            self.file_context.is_callee_from_project(Some(recv))
        } else {
            true
        };

        let resolved_type = receiver_type.map(|rt| {
            if rt.contains('.') {
                rt.to_string()
            } else {
                self.file_context
                    .imports
                    .get(rt)
                    .cloned()
                    .unwrap_or_else(|| {
                        if Self::is_java_lang_type(rt) {
                            format!("java.lang.{}", rt)
                        } else {
                            rt.to_string()
                        }
                    })
            }
        });

        if let Some(ref rtype) = resolved_type {
            if rtype.starts_with("java.") || rtype.starts_with("jakarta.") {
                return false;
            }
            if rtype.starts_with("javax.") {
                return false;
            }
            if rtype.starts_with("org.springframework.") {
                return false;
            }
        }

        if Self::is_getter_setter(callee_name)
            && is_project_type
            && self.is_model_type(receiver_type)
        {
            return false;
        }

        if Self::is_noise_receiver_type(receiver_type) {
            return false;
        }

        true
    }

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

    fn is_getter_setter(name: &str) -> bool {
        (name.starts_with("get")
            && name.len() > 3
            && name.chars().nth(3).is_some_and(|c| c.is_uppercase()))
            || (name.starts_with("is")
                && name.len() > 2
                && name.chars().nth(2).is_some_and(|c| c.is_uppercase()))
            || (name.starts_with("set")
                && name.len() > 3
                && name.chars().nth(3).is_some_and(|c| c.is_uppercase()))
    }

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

    fn is_model_type(&self, receiver_type: Option<&str>) -> bool {
        match receiver_type {
            Some(rtype) => {
                let base = rtype.split('<').next().unwrap_or(rtype);
                let name = base.split('.').last().unwrap_or(base);

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

    fn is_noise_receiver_type(receiver_type: Option<&str>) -> bool {
        match receiver_type {
            Some(rtype) => {
                if rtype.contains(".builder(") || rtype.contains(".build(") {
                    return true;
                }
                if rtype.contains(").") && rtype.chars().any(|c| c == '(' || c == ')') {
                    return true;
                }
                if !rtype.contains('.') && rtype.chars().next().is_some_and(|c| c.is_lowercase()) {
                    return true;
                }
                false
            }
            None => false,
        }
    }
}

pub(super) fn find_callees(
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

    let file_ctx = extract_file_context(tree.root_node(), source.as_bytes());
    let public_language = infer_public_language(path);
    let mut callees = Vec::new();
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

pub(super) struct JavaCalleeContext<'a> {
    pub(super) target_symbol: &'a str,
    pub(super) current_file: &'a Path,
    pub(super) public_language: Option<&'a str>,
    pub(super) file_context: Option<JavaFileContext>,
    pub(super) callee_filter: Option<CalleeFilter>,
    pub(super) active_builder_chains: HashMap<String, String>,
}

#[derive(Clone)]
struct JavaFunctionContext {
    class_name: Option<String>,
}

fn collect_java_callees(
    node: Node,
    source: &[u8],
    current_function: Option<JavaFunctionContext>,
    ctx: &mut JavaCalleeContext,
    callees: &mut Vec<CalleeDefinition>,
) {
    let node_kind = node.kind();
    let is_target_method = matches!(node_kind, "method_declaration" | "constructor_declaration")
        && node
            .child_by_field_name("name")
            .and_then(|n| java_node_text(n, source))
            .map(|name| name == ctx.target_symbol)
            .unwrap_or(false);

    let next_class_name =
        if node.kind() == "class_declaration" || node.kind() == "interface_declaration" {
            node.child_by_field_name("name")
                .and_then(|n| java_node_text(n, source))
                .or_else(|| current_function.as_ref().and_then(|f| f.class_name.clone()))
        } else {
            current_function.as_ref().and_then(|f| f.class_name.clone())
        };

    let next_function = if is_target_method || current_function.is_some() {
        Some(JavaFunctionContext {
            class_name: next_class_name,
        })
    } else {
        current_function.clone()
    };

    if is_target_method || current_function.is_some() {
        if node.kind() == "method_invocation" {
            let receiver_name = node
                .child_by_field_name("object")
                .and_then(|n| java_node_text(n, source));

            let receiver_type = receiver_name.as_ref().and_then(|recv| {
                ctx.file_context
                    .as_ref()
                    .and_then(|fc| fc.class_fields.get(recv).cloned())
            });

            let callee_name = node
                .child_by_field_name("name")
                .and_then(|n| java_node_text(n, source));

            let mut is_builder_chain_call = false;
            if let (Some(recv), Some(name)) = (receiver_name.as_ref(), callee_name.as_ref()) {
                if ctx.active_builder_chains.contains_key(recv) {
                    is_builder_chain_call = true;

                    if name.as_str() == "build" {
                        ctx.active_builder_chains.remove(recv);
                    }
                }

                if name.as_str() == "builder" {
                    let is_model = receiver_type.as_ref().is_some_and(|rt| {
                        let base = rt.split('<').next().unwrap_or(rt);
                        let type_name = base.split('.').last().unwrap_or(base);
                        type_name.ends_with("Builder")
                            || type_name.ends_with("PersistenceModel")
                            || type_name.ends_with("Entity")
                            || (type_name.ends_with("Model") && !type_name.ends_with("ViewModel"))
                            || type_name.ends_with("DTO")
                    });

                    if is_model {
                        let chain_id = format!("{}_builder_{}", recv, node.start_position().row);
                        ctx.active_builder_chains.insert(chain_id, (*recv).clone());
                    }
                }
            }

            if let Some(callee) = extract_java_callee(node, source, ctx, &current_function) {
                if !is_builder_chain_call || callee.callee == "build" {
                    let should_include = ctx.callee_filter.as_ref().map_or(true, |filter| {
                        filter.should_include(
                            &callee.callee,
                            receiver_name.as_deref(),
                            callee.receiver_type.as_deref().or(receiver_type.as_deref()),
                            false,
                        )
                    });

                    if should_include {
                        callees.push(callee);
                    }
                }
            }
        }

        if node.kind() == "object_creation_expression" {
            if let Some(callee) = extract_java_callee(node, source, ctx, &current_function) {
                callees.push(callee);
            }
        }
    }

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

    let end_line = (node.end_position().row + 1) as u32;
    let callee_symbol = current_function
        .as_ref()
        .and_then(|f| f.class_name.as_ref())
        .map(|class| format!("{}#{}", class, callee_name));

    let receiver_type = if let Some(ref file_ctx) = ctx.file_context {
        if let Some(ref receiver) = receiver_name {
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

fn extract_file_context(root: Node, source: &[u8]) -> JavaFileContext {
    let mut package_name = String::new();
    let mut imports: HashMap<String, String> = HashMap::new();
    let mut class_fields: HashMap<String, String> = HashMap::new();

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
                    if let Some(body) = child.child_by_field_name("body") {
                        collect_class_fields(body, source, &mut class_fields);
                    }
                }
                _ => {}
            }
        }
    }

    if package_name.is_empty() {
        package_name = "unknown".to_string();
    }

    let mut ctx = JavaFileContext::new(&package_name);
    ctx.imports = imports;
    ctx.class_fields = class_fields;
    ctx
}

fn extract_import(node: Node, source: &[u8]) -> Option<(String, String)> {
    let name_node = node
        .named_children(&mut node.walk())
        .find(|c| c.kind() == "scoped_identifier" || c.kind() == "identifier")?;

    let full_name = java_node_text(name_node, source)?;
    let simple_name = full_name
        .split('.')
        .last()
        .unwrap_or(&full_name)
        .to_string();

    Some((simple_name, full_name))
}

fn collect_class_fields(body: Node, source: &[u8], fields: &mut HashMap<String, String>) {
    for index in 0..body.named_child_count() {
        if let Some(child) = body.named_child(index) {
            if child.kind() == "field_declaration" {
                extract_field_declaration(child, source, fields);
            }
        }
    }
}

fn extract_field_declaration(node: Node, source: &[u8], fields: &mut HashMap<String, String>) {
    let type_node = node.named_children(&mut node.walk()).find(|c| {
        c.kind() == "type_identifier"
            || c.kind() == "scoped_type_identifier"
            || c.kind() == "generic_type"
    });

    let type_name = type_node.and_then(|n| java_node_text(n, source));

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
