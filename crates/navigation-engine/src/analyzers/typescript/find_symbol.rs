use std::path::Path;

use tree_sitter::{Node, Parser};

use super::super::types::{
    infer_public_language, normalize_public_symbol_kind, FindSymbolQuery, SymbolDefinition,
};
use super::common::{node_text, parser_language_for_path};

pub(super) fn find_symbols(
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

fn collect_symbols(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
    symbols: &mut Vec<SymbolDefinition>,
) {
    if node.kind() == "variable_declarator" {
        if let Some(name_node) = node.child_by_field_name("name") {
            if name_node.kind() == "object_pattern" {
                symbols.extend(extract_object_pattern_symbols(
                    node,
                    name_node,
                    source,
                    public_language,
                ));
            } else if let Some(symbol) = extract_symbol(node, source, public_language) {
                symbols.push(symbol);
            }
        }
    } else if let Some(symbol) = extract_symbol(node, source, public_language) {
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
            let name_node = node.child_by_field_name("name")?;

            if name_node.kind() == "object_pattern" {
                return None;
            }

            let value = node.child_by_field_name("value")?;
            if !matches!(value.kind(), "arrow_function" | "function_expression") {
                return None;
            }
            (name_node, "function_declaration")
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

fn extract_object_pattern_symbols(
    node: Node,
    pattern_node: Node,
    source: &[u8],
    public_language: Option<&str>,
) -> Vec<SymbolDefinition> {
    collect_object_pattern_identifiers(pattern_node, source)
        .into_iter()
        .map(|name| SymbolDefinition {
            symbol: name,
            kind: normalize_public_symbol_kind("function_declaration"),
            path: String::new(),
            line: (node.start_position().row + 1) as u32,
            line_end: (node.end_position().row + 1) as u32,
            language: public_language.map(str::to_string),
        })
        .collect()
}

fn collect_object_pattern_identifiers(node: Node, source: &[u8]) -> Vec<String> {
    let mut names = Vec::new();
    collect_object_pattern_identifiers_recursive(node, source, &mut names);
    names
}

fn collect_object_pattern_identifiers_recursive(
    node: Node,
    source: &[u8],
    names: &mut Vec<String>,
) {
    match node.kind() {
        "shorthand_property_identifier_pattern" | "identifier" => {
            if let Some(name) = node_text(node, source) {
                names.push(name);
            }
            return;
        }
        "pair_pattern" => {
            if let Some(value) = node.child_by_field_name("value") {
                collect_object_pattern_identifiers_recursive(value, source, names);
            }
            return;
        }
        _ => {}
    }

    for index in 0..node.named_child_count() {
        if let Some(child) = node.named_child(index) {
            collect_object_pattern_identifiers_recursive(child, source, names);
        }
    }
}
