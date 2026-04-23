use std::path::Path;

use tree_sitter::{Node, Parser};

use super::super::types::{
    infer_public_language, normalize_public_symbol_kind, FindSymbolQuery, SymbolDefinition,
};
use super::common::node_text;

pub(super) fn find_symbols(
    path: &Path,
    source: &str,
    query: &FindSymbolQuery,
) -> Vec<SymbolDefinition> {
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

    let mut symbols = Vec::new();
    let public_language = infer_public_language(path);
    collect_symbols(
        tree.root_node(),
        source.as_bytes(),
        public_language.as_deref(),
        &mut symbols,
    );

    symbols
        .into_iter()
        .filter(|item| matches_symbol(item, query))
        .take(query.limit)
        .collect()
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
        "record_declaration" => (node.child_by_field_name("name")?, "record_declaration"),
        "enum_declaration" => (node.child_by_field_name("name")?, "enum_declaration"),
        "method_declaration" => (node.child_by_field_name("name")?, "method_declaration"),
        "constructor_declaration" => (node.child_by_field_name("name")?, "constructor_declaration"),
        _ => return None,
    };

    let symbol = node_text(name_node, source)?;
    let symbol = if matches!(raw_kind, "method_declaration" | "constructor_declaration") {
        if let Some(parent) = node.parent() {
            if let Some(grandparent) = parent.parent() {
                if grandparent.kind() == "class_declaration" {
                    if let Some(class_name_node) = grandparent.child_by_field_name("name") {
                        if let Some(class_name) = node_text(class_name_node, source) {
                            format!("{}.{}", class_name, symbol)
                        } else {
                            symbol
                        }
                    } else {
                        symbol
                    }
                } else {
                    symbol
                }
            } else {
                symbol
            }
        } else {
            symbol
        }
    } else {
        symbol
    };

    Some(SymbolDefinition {
        symbol,
        kind: normalize_public_symbol_kind(raw_kind),
        path: String::new(),
        line: (node.start_position().row + 1) as u32,
        line_end: (node.end_position().row + 1) as u32,
        language: public_language.map(str::to_string),
    })
}

fn matches_symbol(item: &SymbolDefinition, query: &FindSymbolQuery) -> bool {
    let symbol_match = match query.match_mode.as_str() {
        "fuzzy" => item.symbol.to_lowercase().contains(&query.symbol.to_lowercase()),
        _ => {
            if item.symbol == query.symbol {
                true
            } else {
                // If query is a simple name, match the suffix after the dot
                item.symbol.ends_with(&format!(".{}", query.symbol))
            }
        }
    };

    let kind_match = query.kind == "any" || item.kind == query.kind;
    let language_match = query
        .public_language_filter
        .as_ref()
        .is_none_or(|language| item.language.as_deref() == Some(language.as_str()));

    symbol_match && kind_match && language_match
}
