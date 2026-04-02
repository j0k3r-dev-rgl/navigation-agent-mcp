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
            "struct_item" => push_named_symbol(child, source, public_language, "type", symbols),
            "enum_item" => push_named_symbol(child, source, public_language, "enum", symbols),
            "trait_item" => push_named_symbol(child, source, public_language, "interface", symbols),
            "type_item" => push_named_symbol(child, source, public_language, "type", symbols),
            "function_item" => {
                push_named_symbol(child, source, public_language, "function", symbols)
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
    let Some(body) = impl_body(impl_item) else {
        return;
    };

    for index in 0..body.named_child_count() {
        let Some(child) = body.named_child(index) else {
            continue;
        };

        if child.kind() == "function_item" {
            push_named_symbol(child, source, public_language, "method", symbols);
        }
    }
}

fn push_named_symbol(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
    kind: &str,
    symbols: &mut Vec<SymbolDefinition>,
) {
    if let Some(symbol) = build_named_symbol(node, source, public_language, kind) {
        symbols.push(symbol);
    }
}

fn build_named_symbol(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
    kind: &str,
) -> Option<SymbolDefinition> {
    let name_node = node.child_by_field_name("name")?;

    Some(SymbolDefinition {
        symbol: node_text(name_node, source)?,
        kind: kind.to_string(),
        path: String::new(),
        line: (node.start_position().row + 1) as u32,
        line_end: (node.end_position().row + 1) as u32,
        language: public_language.map(str::to_string),
    })
}
