use std::path::Path;

use tree_sitter::{Node, Parser};

use super::super::types::{infer_public_language, FindSymbolQuery, SymbolDefinition};
use super::common::{go_symbol_matches_target, node_text, simplify_go_type_name};

pub(super) fn find_symbols(
    path: &Path,
    source: &str,
    query: &FindSymbolQuery,
) -> Vec<SymbolDefinition> {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_go::LANGUAGE.into())
        .is_err()
    {
        return Vec::new();
    }

    let Some(tree) = parser.parse(source, None) else {
        return Vec::new();
    };

    let public_language = infer_public_language(path);
    let mut symbols = Vec::new();
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
    match node.kind() {
        "function_declaration" => {
            if let Some(symbol) = function_symbol(node, source, public_language) {
                symbols.push(symbol);
            }
        }
        "method_declaration" => {
            if let Some(symbol) = method_symbol(node, source, public_language) {
                symbols.push(symbol);
            }
        }
        "type_declaration" => {
            collect_type_declaration_symbols(node, source, public_language, symbols)
        }
        _ => {}
    }

    for index in 0..node.named_child_count() {
        if let Some(child) = node.named_child(index) {
            collect_symbols(child, source, public_language, symbols);
        }
    }
}

fn function_symbol(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
) -> Option<SymbolDefinition> {
    let name_node = node.child_by_field_name("name").or_else(|| {
        (0..node.named_child_count())
            .filter_map(|index| node.named_child(index))
            .find(|child| child.kind() == "identifier")
    })?;
    Some(SymbolDefinition {
        symbol: node_text(name_node, source)?,
        kind: "function".to_string(),
        path: String::new(),
        line: (node.start_position().row + 1) as u32,
        line_end: (node.end_position().row + 1) as u32,
        language: public_language.map(str::to_string),
    })
}

fn method_symbol(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
) -> Option<SymbolDefinition> {
    let name_node = node.child_by_field_name("name").or_else(|| {
        (0..node.named_child_count())
            .filter_map(|index| node.named_child(index))
            .find(|child| child.kind() == "field_identifier")
    })?;
    let method_name = node_text(name_node, source)?;
    let receiver = node.child_by_field_name("receiver").or_else(|| {
        (0..node.named_child_count())
            .filter_map(|index| node.named_child(index))
            .find(|child| child.kind() == "parameter_list")
    })?;
    let owner = extract_receiver_type(receiver, source)?;

    Some(SymbolDefinition {
        symbol: format!("{}.{}", owner, method_name),
        kind: "method".to_string(),
        path: String::new(),
        line: (node.start_position().row + 1) as u32,
        line_end: (node.end_position().row + 1) as u32,
        language: public_language.map(str::to_string),
    })
}

fn collect_type_declaration_symbols(
    node: Node,
    source: &[u8],
    public_language: Option<&str>,
    symbols: &mut Vec<SymbolDefinition>,
) {
    for index in 0..node.named_child_count() {
        let Some(child) = node.named_child(index) else {
            continue;
        };
        if !matches!(child.kind(), "type_spec" | "type_alias") {
            continue;
        }
        let Some(name_node) = (0..child.named_child_count())
            .filter_map(|inner| child.named_child(inner))
            .find(|candidate| candidate.kind() == "type_identifier")
        else {
            continue;
        };
        let type_node = if child.kind() == "type_alias" {
            child.named_child(1)
        } else {
            child.child_by_field_name("type").or_else(|| {
                (0..child.named_child_count())
                    .filter_map(|inner| child.named_child(inner))
                    .find(|candidate| candidate.kind() != "type_identifier")
            })
        };
        let Some(type_node) = type_node else {
            continue;
        };
        let kind = match type_node.kind() {
            "interface_type" => "interface",
            "struct_type" => "class",
            "type_alias" => "type",
            _ => "type",
        };
        if let Some(name) = node_text(name_node, source) {
            symbols.push(SymbolDefinition {
                symbol: name,
                kind: kind.to_string(),
                path: String::new(),
                line: (child.start_position().row + 1) as u32,
                line_end: (child.end_position().row + 1) as u32,
                language: public_language.map(str::to_string),
            });
        }
    }
}

fn extract_receiver_type(receiver: Node, source: &[u8]) -> Option<String> {
    for index in 0..receiver.named_child_count() {
        let child = receiver.named_child(index)?;
        if child.kind() == "parameter_declaration" {
            for inner in 0..child.named_child_count() {
                let candidate = child.named_child(inner)?;
                if matches!(
                    candidate.kind(),
                    "type_identifier" | "qualified_type" | "pointer_type"
                ) {
                    return node_text(candidate, source).map(|value| simplify_go_type_name(&value));
                }
            }
        }
        if matches!(
            child.kind(),
            "type_identifier" | "qualified_type" | "pointer_type"
        ) {
            return node_text(child, source).map(|value| simplify_go_type_name(&value));
        }
    }
    node_text(receiver, source).map(|value| simplify_go_type_name(&value))
}

fn matches_symbol(item: &SymbolDefinition, query: &FindSymbolQuery) -> bool {
    let symbol_match = match query.match_mode.as_str() {
        "fuzzy" => item.symbol.contains(&query.symbol),
        _ => go_symbol_matches_target(&item.symbol, &query.symbol),
    };

    let kind_match = query.kind == "any" || item.kind == query.kind;
    let language_match = query
        .public_language_filter
        .as_ref()
        .is_none_or(|language| item.language.as_deref() == Some(language.as_str()));

    symbol_match && kind_match && language_match
}
