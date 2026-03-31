use std::path::Path;

use tree_sitter::{Node, Parser};

use super::language_analyzer::LanguageAnalyzer;
use super::types::{infer_public_language, AnalyzerLanguage, FindSymbolQuery, SymbolDefinition};

pub struct RustAnalyzer;

impl LanguageAnalyzer for RustAnalyzer {
    fn language(&self) -> AnalyzerLanguage {
        AnalyzerLanguage::Rust
    }

    fn supported_extensions(&self) -> &'static [&'static str] {
        &[".rs"]
    }

    fn find_symbols(
        &self,
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

fn impl_body(node: Node) -> Option<Node> {
    node.child_by_field_name("body").or_else(|| {
        (0..node.named_child_count())
            .filter_map(|index| node.named_child(index))
            .find(|child| child.kind() == "declaration_list")
    })
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
        language: public_language.map(str::to_string),
    })
}

fn node_text(node: Node, source: &[u8]) -> Option<String> {
    node.utf8_text(source)
        .ok()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::FindSymbolQuery;

    fn any_query() -> FindSymbolQuery {
        FindSymbolQuery {
            symbol: "load".to_string(),
            kind: "any".to_string(),
            match_mode: "fuzzy".to_string(),
            public_language_filter: None,
            limit: 50,
        }
    }

    #[test]
    fn extracts_supported_rust_definitions() {
        let analyzer = RustAnalyzer;
        let source = r#"
pub struct UserId;
pub enum JobState { Ready }
pub trait Runner {
    fn run(&self);
}
pub type LoadResult = String;
pub fn load() {}

impl UserId {
    pub fn new() -> Self { UserId }

    #[cfg(test)]
    pub fn load_cached(&self) {}
}
"#;

        let items = analyzer.find_symbols(Path::new("src/lib.rs"), source, &any_query());
        let kinds = items
            .iter()
            .map(|item| {
                (
                    item.symbol.as_str(),
                    item.kind.as_str(),
                    item.language.as_deref(),
                )
            })
            .collect::<Vec<_>>();

        assert!(kinds.contains(&("UserId", "type", Some("rust"))));
        assert!(kinds.contains(&("JobState", "enum", Some("rust"))));
        assert!(kinds.contains(&("Runner", "interface", Some("rust"))));
        assert!(kinds.contains(&("LoadResult", "type", Some("rust"))));
        assert!(kinds.contains(&("load", "function", Some("rust"))));
        assert!(kinds.contains(&("new", "method", Some("rust"))));
        assert!(kinds.contains(&("load_cached", "method", Some("rust"))));
        assert_eq!(
            items
                .iter()
                .filter(|item| item.symbol == "load_cached")
                .count(),
            1
        );
    }

    #[test]
    fn excludes_unsupported_rust_constructs() {
        let analyzer = RustAnalyzer;
        let source = r#"
macro_rules! nope { () => {} }
mod nested {}
union Bits { value: u32 }
const LIMIT: u32 = 1;
static NAME: &str = "x";
extern "C" { fn ffi(); }

trait Shape {
    fn area(&self);
    const SIDES: usize;
    type Output;
}

fn outer() {
    fn inner() {}
}
"#;

        let items = analyzer.find_symbols(Path::new("src/unsupported.rs"), source, &any_query());
        let names = items
            .iter()
            .map(|item| item.symbol.as_str())
            .collect::<Vec<_>>();

        assert_eq!(names, vec!["Shape", "outer"]);
        assert!(!names.contains(&"ffi"));
        assert!(!names.contains(&"area"));
        assert!(!names.contains(&"SIDES"));
        assert!(!names.contains(&"Output"));
        assert!(!names.contains(&"inner"));
    }
}
