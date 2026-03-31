use std::collections::BTreeSet;
use std::path::Path;

use super::java::JavaAnalyzer;
use super::language_analyzer::LanguageAnalyzer;
use super::python::PythonAnalyzer;
use super::rust::RustAnalyzer;
use super::types::{file_extension, AnalyzerLanguage};
use super::typescript::TypeScriptAnalyzer;

#[derive(Default)]
pub struct AnalyzerRegistry {
    analyzers: Vec<Box<dyn LanguageAnalyzer + Send + Sync>>,
}

impl AnalyzerRegistry {
    pub fn new() -> Self {
        let mut registry = Self::default();
        registry.register(Box::new(JavaAnalyzer));
        registry.register(Box::new(PythonAnalyzer));
        registry.register(Box::new(RustAnalyzer));
        registry.register(Box::new(TypeScriptAnalyzer));
        registry
    }

    pub fn register(&mut self, analyzer: Box<dyn LanguageAnalyzer + Send + Sync>) {
        self.analyzers.push(analyzer);
    }

    pub fn supported_extensions(&self, language: AnalyzerLanguage) -> BTreeSet<String> {
        self.analyzers
            .iter()
            .filter(|analyzer| {
                language == AnalyzerLanguage::Auto || analyzer.language() == language
            })
            .flat_map(|analyzer| analyzer.supported_extensions().iter().copied())
            .map(|value| value.to_string())
            .collect()
    }

    pub fn analyzer_for_file(
        &self,
        language: AnalyzerLanguage,
        path: &Path,
    ) -> Option<&(dyn LanguageAnalyzer + Send + Sync)> {
        let extension = file_extension(path)?;

        self.analyzers
            .iter()
            .find(|analyzer| {
                (language == AnalyzerLanguage::Auto || analyzer.language() == language)
                    && analyzer
                        .supported_extensions()
                        .contains(&extension.as_str())
            })
            .map(|analyzer| analyzer.as_ref())
    }
}
