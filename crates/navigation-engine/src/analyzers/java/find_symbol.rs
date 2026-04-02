use std::path::Path;

use tree_sitter::{Node, Parser};

use super::super::types::{
    infer_public_language, normalize_public_symbol_kind, FindSymbolQuery, SymbolDefinition,
};
use super::common::node_text;

pub(super) fn find_symbols(
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
