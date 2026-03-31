use std::path::Path;

use tree_sitter::{Node, Parser};

use super::language_analyzer::LanguageAnalyzer;
use super::types::{
    infer_public_language, normalize_public_symbol_kind, AnalyzerLanguage, FindSymbolQuery,
    SymbolDefinition,
};

pub struct TypeScriptAnalyzer;

impl LanguageAnalyzer for TypeScriptAnalyzer {
    fn language(&self) -> AnalyzerLanguage {
        AnalyzerLanguage::Typescript
    }

    fn supported_extensions(&self) -> &'static [&'static str] {
        &[".ts", ".tsx", ".js", ".jsx"]
    }

    fn find_symbols(&self, path: &Path, source: &str, _query: &FindSymbolQuery) -> Vec<SymbolDefinition> {
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
}

fn parser_language_for_path(path: &Path) -> Option<tree_sitter_language::LanguageFn> {
    match path.extension().and_then(|value| value.to_str()).map(|value| value.to_ascii_lowercase()) {
        Some(extension) if extension == "ts" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT),
        Some(extension) if extension == "tsx" => Some(tree_sitter_typescript::LANGUAGE_TSX),
        Some(extension) if extension == "js" || extension == "jsx" => Some(tree_sitter_javascript::LANGUAGE),
        _ => None,
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
                language: public_language.map(str::to_string),
            });
        }
        "variable_declarator" => {
            let value = node.child_by_field_name("value")?;
            if !matches!(value.kind(), "arrow_function" | "function_expression") {
                return None;
            }
            (node.child_by_field_name("name")?, "function_declaration")
        }
        _ => return None,
    };

    Some(SymbolDefinition {
        symbol: node_text(name_node, source)?,
        kind: normalize_public_symbol_kind(raw_kind),
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
    use std::path::Path;

    fn any_query() -> FindSymbolQuery {
        FindSymbolQuery {
            symbol: "loader".to_string(),
            kind: "any".to_string(),
            match_mode: "exact".to_string(),
            public_language_filter: None,
            limit: 50,
        }
    }

    #[test]
    fn extracts_typescript_definitions_with_public_kinds() {
        let analyzer = TypeScriptAnalyzer;
        let source = r#"
interface LoaderArgs { value: string }
type LoaderResult = string;
enum Mode { A, B }
class Worker {
  constructor() {}
  run() {}
}
function loader() {}
const action = () => {};
"#;

        let items = analyzer.find_symbols(Path::new("src/routes/example.ts"), source, &any_query());
        let kinds = items
            .iter()
            .map(|item| (item.symbol.as_str(), item.kind.as_str(), item.language.as_deref()))
            .collect::<Vec<_>>();

        assert!(kinds.contains(&("LoaderArgs", "interface", Some("typescript"))));
        assert!(kinds.contains(&("LoaderResult", "type", Some("typescript"))));
        assert!(kinds.contains(&("Mode", "enum", Some("typescript"))));
        assert!(kinds.contains(&("Worker", "class", Some("typescript"))));
        assert!(kinds.contains(&("constructor", "constructor", Some("typescript"))));
        assert!(kinds.contains(&("run", "method", Some("typescript"))));
        assert!(kinds.contains(&("loader", "function", Some("typescript"))));
        assert!(kinds.contains(&("action", "function", Some("typescript"))));
    }

    #[test]
    fn extracts_javascript_definitions_with_javascript_language() {
        let analyzer = TypeScriptAnalyzer;
        let source = r#"
class Widget {
  render() {}
}
const loader = () => {};
function action() {}
"#;

        let items = analyzer.find_symbols(Path::new("src/routes/example.js"), source, &any_query());
        let kinds = items
            .iter()
            .map(|item| (item.symbol.as_str(), item.kind.as_str(), item.language.as_deref()))
            .collect::<Vec<_>>();

        assert!(kinds.contains(&("Widget", "class", Some("javascript"))));
        assert!(kinds.contains(&("render", "method", Some("javascript"))));
        assert!(kinds.contains(&("loader", "function", Some("javascript"))));
        assert!(kinds.contains(&("action", "function", Some("javascript"))));
    }
}
