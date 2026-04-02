use std::path::Path;

use std::collections::BTreeSet;

use tree_sitter::{Node, Parser};

use super::super::types::{infer_public_language, FindSymbolQuery, SymbolDefinition};
use super::common::node_text;

pub(super) fn find_symbols(
    path: &Path,
    source: &str,
    _query: &FindSymbolQuery,
) -> Vec<SymbolDefinition> {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
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

    dedupe_symbols(symbols)
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
    let effective_node = unwrap_decorated_definition(node)?;
    let raw_kind = match effective_node.kind() {
        "class_definition" => "class",
        "function_definition" | "async_function_definition" => {
            classify_function_kind(effective_node)?
        }
        _ => return None,
    };

    let name_node = effective_node.child_by_field_name("name")?;

    Some(SymbolDefinition {
        symbol: node_text(name_node, source)?,
        kind: raw_kind.to_string(),
        path: String::new(),
        line: (effective_node.start_position().row + 1) as u32,
        line_end: (effective_node.end_position().row + 1) as u32,
        language: public_language.map(str::to_string),
    })
}

fn unwrap_decorated_definition(node: Node) -> Option<Node> {
    if node.kind() != "decorated_definition" {
        return Some(node);
    }

    for index in 0..node.named_child_count() {
        let child = node.named_child(index)?;
        if child.kind() != "decorator" {
            return Some(child);
        }
    }

    None
}

fn classify_function_kind(node: Node) -> Option<&'static str> {
    let mut current = node.parent();
    let mut inside_class = false;

    while let Some(parent) = current {
        match parent.kind() {
            "function_definition" | "async_function_definition" => return None,
            "class_definition" => inside_class = true,
            _ => {}
        }
        current = parent.parent();
    }

    Some(if inside_class { "method" } else { "function" })
}

fn dedupe_symbols(symbols: Vec<SymbolDefinition>) -> Vec<SymbolDefinition> {
    let mut seen = BTreeSet::new();
    let mut deduped = Vec::new();

    for symbol in symbols {
        let key = (
            symbol.symbol.clone(),
            symbol.kind.clone(),
            symbol.line,
            symbol.language.clone(),
        );

        if seen.insert(key) {
            deduped.push(symbol);
        }
    }

    deduped
}
