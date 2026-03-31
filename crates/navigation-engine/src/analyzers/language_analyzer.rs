use std::path::Path;

use super::types::{AnalyzerLanguage, FindSymbolQuery, SymbolDefinition};

pub trait LanguageAnalyzer {
    fn language(&self) -> AnalyzerLanguage;

    fn supported_extensions(&self) -> &'static [&'static str];

    fn find_symbols(&self, _path: &Path, _source: &str, _query: &FindSymbolQuery) -> Vec<SymbolDefinition> {
        Vec::new()
    }
}
