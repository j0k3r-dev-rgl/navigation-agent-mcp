use std::path::Path;

use tree_sitter::{Node, Parser};

use super::super::types::{infer_public_language, FindSymbolQuery, SymbolDefinition};
use super::common::{impl_body, node_text};

pub(super) fn find_symbols(
    path: &Path,
    source: &str,
    _query: &FindSymbolQuery,
) -> Vec<SymbolDefinition> {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .is_err()
    {
        return Vec::new();
    }

    let Some(tree) = parser.parse(source, None) else {
        return Vec::new();
    };

    let public_language = infer_public_language(path);
    let mut symbols = Vec::new();
    collect_source_file_symbols(
        tree.root_node(),
        source.as_bytes(),
        public_language.as_deref(),
        &mut symbols,
    );
    symbols
}

fn collect_source_file_symbols(
    root: Node,
    source: &[u8],
    public_language: Option<&str>,
    symbols: &mut Vec<SymbolDefinition>,
) {
    for index in 0..root.named_child_count() {
        let Some(child) = root.named_child(index) else {
            continue;
        };

        match child.kind() {
            "struct_item" => {
                push_named_symbol(child, source, public_language, "type", None, symbols)
            }
            "enum_item" => push_named_symbol(child, source, public_language, "enum", None, symbols),
            "trait_item" => {
                push_named_symbol(child, source, public_language, "interface", None, symbols)
            }
            "type_item" => push_named_symbol(child, source, public_language, "type", None, symbols),
            "function_item" => {
                push_named_symbol(child, source, public_language, "function", None, symbols)
            }
            "impl_item" => collect_impl_methods(child, source, public_language, symbols),
            _ => {}
        }
    }
}

fn collect_impl_methods(
    impl_item: Node,
    source: &[u8],
    public_language: Option<&str>,
    symbols: &mut Vec<SymbolDefinition>,
) {
    let impl_owner = extract_impl_owner_name(impl_item, source);
    let Some(body) = impl_body(impl_item) else {
        return;
    };

    for index in 0..body.named_child_count() {
        let Some(child) = body.named_child(index) else {
            continue;
        };

        if child.kind() == "function_item" {
            push_named_symbol(
                child,
                source,
                public_language,
                "method",
                impl_owner.as_deref(),
                symbols,
            );
        }
    }
}

fn push_named_symbol(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
    kind: &str,
    owner_name: Option<&str>,
    symbols: &mut Vec<SymbolDefinition>,
) {
    if let Some(symbol) = build_named_symbol(node, source, public_language, kind, owner_name) {
        symbols.push(symbol);
    }
}

fn build_named_symbol(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
    kind: &str,
    owner_name: Option<&str>,
) -> Option<SymbolDefinition> {
    let name_node = node.child_by_field_name("name")?;
    let base_name = node_text(name_node, source)?;
    let symbol_name = if kind == "method" {
        owner_name
            .map(|owner| format!("{}::{}", owner, base_name))
            .unwrap_or(base_name)
    } else {
        base_name
    };

    Some(SymbolDefinition {
        symbol: symbol_name,
        kind: kind.to_string(),
        path: String::new(),
        line: (node.start_position().row + 1) as u32,
        line_end: (node.end_position().row + 1) as u32,
        language: public_language.map(str::to_string),
    })
}

fn extract_impl_owner_name(impl_item: Node, source: &[u8]) -> Option<String> {
    if let Some(type_node) = impl_item.child_by_field_name("type") {
        return node_text(type_node, source)
            .map(|value| simplify_rust_type_name(&value))
            .filter(|value| !value.is_empty());
    }

    for index in 0..impl_item.named_child_count() {
        let child = impl_item.named_child(index)?;
        if matches!(
            child.kind(),
            "type_identifier"
                | "scoped_type_identifier"
                | "generic_type"
                | "tuple_type"
                | "reference_type"
                | "primitive_type"
        ) {
            if let Some(value) = node_text(child, source) {
                let simplified = simplify_rust_type_name(&value);
                if !simplified.is_empty() {
                    return Some(simplified);
                }
            }
        }
    }

    None
}

fn simplify_rust_type_name(value: &str) -> String {
    let trimmed = value.trim();
    let without_ref = trimmed
        .trim_start_matches('&')
        .trim_start_matches("mut ")
        .trim();
    let base = without_ref.split('<').next().unwrap_or(without_ref).trim();
    base.rsplit("::").next().unwrap_or(base).trim().to_string()
}
