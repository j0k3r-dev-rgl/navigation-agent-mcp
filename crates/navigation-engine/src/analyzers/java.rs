use std::path::Path;

use tree_sitter::{Node, Parser};

use super::language_analyzer::LanguageAnalyzer;
use super::types::{
    infer_public_language, normalize_public_symbol_kind, AnalyzerLanguage, FindSymbolQuery,
    SymbolDefinition,
};

pub struct JavaAnalyzer;

impl LanguageAnalyzer for JavaAnalyzer {
    fn language(&self) -> AnalyzerLanguage {
        AnalyzerLanguage::Java
    }

    fn supported_extensions(&self) -> &'static [&'static str] {
        &[".java"]
    }

    fn find_symbols(&self, path: &Path, source: &str, _query: &FindSymbolQuery) -> Vec<SymbolDefinition> {
        let mut parser = Parser::new();
        if parser.set_language(&tree_sitter_java::LANGUAGE.into()).is_err() {
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
        "annotation_type_declaration" => {
            (node.child_by_field_name("name")?, "annotation_type")
        }
        "record_declaration" => (node.child_by_field_name("name")?, "record"),
        "method_declaration" => (node.child_by_field_name("name")?, "method_declaration"),
        "constructor_declaration" => {
            (node.child_by_field_name("name")?, "constructor_declaration")
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

    fn any_query() -> FindSymbolQuery {
        FindSymbolQuery {
            symbol: "Example".to_string(),
            kind: "any".to_string(),
            match_mode: "exact".to_string(),
            public_language_filter: None,
            limit: 50,
        }
    }

    #[test]
    fn extracts_java_definitions_with_public_kinds() {
        let analyzer = JavaAnalyzer;
        let source = r#"
package demo;

public @interface Audit {}

public interface ExamplePort {
    void execute();
}

public enum Status {
    ACTIVE,
    INACTIVE
}

public record ExampleRecord(String value) {}

public class ExampleService {
    public ExampleService() {}
    public void execute() {}
}
"#;

        let items = analyzer.find_symbols(Path::new("src/main/java/demo/ExampleService.java"), source, &any_query());
        let kinds = items
            .iter()
            .map(|item| (item.symbol.as_str(), item.kind.as_str(), item.language.as_deref()))
            .collect::<Vec<_>>();

        assert!(kinds.contains(&("Audit", "annotation", Some("java"))));
        assert!(kinds.contains(&("ExamplePort", "interface", Some("java"))));
        assert!(kinds.contains(&("Status", "enum", Some("java"))));
        assert!(kinds.contains(&("ExampleRecord", "type", Some("java"))));
        assert!(kinds.contains(&("ExampleService", "class", Some("java"))));
        assert!(kinds.contains(&("ExampleService", "constructor", Some("java"))));
        assert!(kinds.contains(&("execute", "method", Some("java"))));
    }
}
