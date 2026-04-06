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
        .set_language(&tree_sitter_php::LANGUAGE_PHP.into())
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
    let raw_kind = match node.kind() {
        "class_declaration" => "class",
        "interface_declaration" => "interface",
        "trait_declaration" => "type",
        "function_definition" => classify_function_kind(node)?,
        "method_declaration" => "method",
        "enum_declaration" => "enum",
        _ => return None,
    };

    let name_node = node.child_by_field_name("name")?;

    Some(SymbolDefinition {
        symbol: node_text(name_node, source)?,
        kind: raw_kind.to_string(),
        path: String::new(),
        line: (node.start_position().row + 1) as u32,
        line_end: (node.end_position().row + 1) as u32,
        language: public_language.map(str::to_string),
    })
}

fn classify_function_kind(node: Node) -> Option<&'static str> {
    let mut current = node.parent();
    let mut inside_class = false;

    while let Some(parent) = current {
        if matches!(
            parent.kind(),
            "class_declaration" | "interface_declaration" | "trait_declaration"
        ) {
            inside_class = true;
            break;
        }
        current = parent.parent();
    }

    if inside_class {
        Some("method")
    } else {
        Some("function")
    }
}

fn dedupe_symbols(mut symbols: Vec<SymbolDefinition>) -> Vec<SymbolDefinition> {
    let mut seen = BTreeSet::new();
    symbols.retain(|symbol| {
        let key = (symbol.line, symbol.symbol.clone());
        seen.insert(key)
    });
    symbols
}
