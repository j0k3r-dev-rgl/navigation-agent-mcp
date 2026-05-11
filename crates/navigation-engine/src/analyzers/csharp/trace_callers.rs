use std::path::Path;

use tree_sitter::{Node, Parser};

use crate::tree_sitter_ext::NodeExt;
use super::super::types::{
    infer_public_language, CallerCallSite, CallerDefinition, CallerRange, CallerTarget,
    FindCallersQuery,
};
use super::common::node_text;

pub(super) fn find_callers(
    _workspace_root: &Path,
    path: &Path,
    source: &str,
    query: &FindCallersQuery,
) -> Vec<CallerDefinition> {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_c_sharp::LANGUAGE.into())
        .is_err()
    {
        return Vec::new();
    }

    let Some(tree) = parser.parse(source, None) else {
        return Vec::new();
    };

    let public_language = infer_public_language(path);
    let mut callers = Vec::new();
    collect_csharp_callers(
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

fn collect_csharp_callers(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
    query: &FindCallersQuery,
    current_class: Option<String>,
    current_method: Option<(String, Vec<String>, CallerRange)>,
    callers: &mut Vec<CallerDefinition>,
) {
    let next_class = if matches!(
        node.kind(),
        "class_declaration" | "interface_declaration" | "record_declaration" | "struct_declaration"
    ) {
        node.child_by_field_name("name")
            .and_then(|item| node_text(item, source))
            .or(current_class.clone())
    } else {
        current_class.clone()
    };

    let next_method = if matches!(
        node.kind(),
        "method_declaration" | "constructor_declaration" | "destructor_declaration"
    ) {
        let name = node
            .child_by_field_name("name")
            .and_then(|item| node_text(item, source));
        name.map(|method_name| {
            let caller_display = next_class
                .as_ref()
                .map(|class_name| format!("{}.{}", class_name, method_name))
                .unwrap_or_else(|| method_name.clone());
            (
                caller_display,
                Vec::new(), // No entry point reasons for C# yet
                CallerRange {
                    start_line: (node.start_position().row + 1) as u32,
                    end_line: (node.end_position().row + 1) as u32,
                },
            )
        })
    } else {
        current_method.clone()
    };

    if node.kind() == "invocation_expression" {
        if let Some(caller) = extract_csharp_call(node, source, public_language, query, &next_method)
        {
            callers.push(caller);
        }
    }

    for index in 0..node.named_child_count() {
        if let Some(child) = node.named_child_at(index) {
            collect_csharp_callers(
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

fn extract_csharp_call(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
    query: &FindCallersQuery,
    current_method: &Option<(String, Vec<String>, CallerRange)>,
) -> Option<CallerDefinition> {
    let function = node.child_by_field_name("function")?;
    
    let (name, receiver_type) = match function.kind() {
        "member_access_expression" => {
            let name = function
                .child_by_field_name("name")
                .and_then(|item| node_text(item, source))?;
            let receiver = function
                .child_by_field_name("expression")
                .and_then(|item| node_text(item, source));
            (name, receiver)
        }
        "identifier" => (node_text(function, source)?, None),
        _ => return None,
    };

    if name != query.target_symbol {
        // Handle symbol like "Class.Method"
        if !query.target_symbol.ends_with(&format!(".{}", name)) {
             return None;
        }
    }

    let (caller_display, reasons, caller_range) = current_method.as_ref()?.clone();

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
        caller_range,
        call_site: CallerCallSite {
            line: (node.start_position().row + 1) as u32,
            column: Some((node.start_position().column + 1) as u32),
            relation: "calls".to_string(),
            snippet: node_text(node, source),
            receiver_type: receiver_type.clone(),
        },
        calls: CallerTarget {
            path: query.target_path.to_string_lossy().replace('\\', "/"),
            symbol: query.target_symbol.clone(),
        },
        probable_entry_point_reasons: reasons,
    })
}
