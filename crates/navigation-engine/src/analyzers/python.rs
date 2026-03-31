use std::collections::BTreeSet;
use std::path::Path;

use tree_sitter::{Node, Parser};

use super::language_analyzer::LanguageAnalyzer;
use super::types::{infer_public_language, AnalyzerLanguage, FindSymbolQuery, SymbolDefinition};

pub struct PythonAnalyzer;

impl LanguageAnalyzer for PythonAnalyzer {
    fn language(&self) -> AnalyzerLanguage {
        AnalyzerLanguage::Python
    }

    fn supported_extensions(&self) -> &'static [&'static str] {
        &[".py"]
    }

    fn find_symbols(&self, path: &Path, source: &str, _query: &FindSymbolQuery) -> Vec<SymbolDefinition> {
        let mut parser = Parser::new();
        if parser.set_language(&tree_sitter_python::LANGUAGE.into()).is_err() {
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
        "function_definition" | "async_function_definition" => classify_function_kind(effective_node)?,
        _ => return None,
    };

    let name_node = effective_node.child_by_field_name("name")?;

    Some(SymbolDefinition {
        symbol: node_text(name_node, source)?,
        kind: raw_kind.to_string(),
        path: String::new(),
        line: (effective_node.start_position().row + 1) as u32,
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
    fn extracts_supported_python_definitions() {
        let analyzer = PythonAnalyzer;
        let source = r#"
class Worker:
    @classmethod
    def build(cls):
        return cls()

    async def fetch(self):
        return 1

def load_data():
    return 1

async def fetch_users():
    return []

@decorator
def serialize():
    return "ok"

@decorator
class DecoratedService:
    pass

class DecoratedMethods:
    @staticmethod
    @audit
    def save():
        return True
"#;

        let items = analyzer.find_symbols(Path::new("profiles/service.py"), source, &any_query());
        let kinds = items
            .iter()
            .map(|item| (item.symbol.as_str(), item.kind.as_str(), item.language.as_deref()))
            .collect::<Vec<_>>();

        assert!(kinds.contains(&("Worker", "class", Some("python"))));
        assert!(kinds.contains(&("build", "method", Some("python"))));
        assert!(kinds.contains(&("fetch", "method", Some("python"))));
        assert!(kinds.contains(&("load_data", "function", Some("python"))));
        assert!(kinds.contains(&("fetch_users", "function", Some("python"))));
        assert!(kinds.contains(&("serialize", "function", Some("python"))));
        assert!(kinds.contains(&("DecoratedService", "class", Some("python"))));
        assert!(kinds.contains(&("save", "method", Some("python"))));

        assert_eq!(items.iter().filter(|item| item.symbol == "serialize").count(), 1);
        assert_eq!(items.iter().filter(|item| item.symbol == "DecoratedService").count(), 1);
        assert_eq!(items.iter().filter(|item| item.symbol == "save").count(), 1);
    }

    #[test]
    fn excludes_unsupported_python_constructs() {
        let analyzer = PythonAnalyzer;
        let source = r#"
value = lambda: 1
from users import load_alias

def outer():
    def inner():
        return 1
    return inner
"#;

        let items = analyzer.find_symbols(Path::new("profiles/unsupported.py"), source, &any_query());
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].symbol, "outer");
        assert_eq!(items[0].kind, "function");
        assert!(items.iter().all(|item| item.symbol != "inner"));
        assert!(items.iter().all(|item| item.symbol != "load_alias"));
    }
}
