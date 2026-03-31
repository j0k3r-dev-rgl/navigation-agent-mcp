use std::path::Path;

use super::types::{
    AnalyzerLanguage, CallerDefinition, EndpointDefinition, FindCallersQuery, FindEndpointsQuery,
    FindSymbolQuery, SymbolDefinition,
};

pub trait LanguageAnalyzer {
    fn language(&self) -> AnalyzerLanguage;

    fn supported_extensions(&self) -> &'static [&'static str];

    fn find_symbols(
        &self,
        _path: &Path,
        _source: &str,
        _query: &FindSymbolQuery,
    ) -> Vec<SymbolDefinition> {
        Vec::new()
    }

    fn find_endpoints(
        &self,
        _path: &Path,
        _source: &str,
        _query: &FindEndpointsQuery,
    ) -> Vec<EndpointDefinition> {
        Vec::new()
    }

    fn find_callers(
        &self,
        _workspace_root: &Path,
        _path: &Path,
        _source: &str,
        _query: &FindCallersQuery,
    ) -> Vec<CallerDefinition> {
        Vec::new()
    }

    fn supports_framework(&self, _framework: Option<&str>) -> bool {
        true
    }
}
