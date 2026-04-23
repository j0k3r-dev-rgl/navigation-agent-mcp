use std::path::Path;

use tree_sitter::Node;

use super::super::language_analyzer::LanguageAnalyzer;
use super::super::types::{
    AnalyzerLanguage, CalleeDefinition, CallerDefinition, EndpointDefinition, FindCalleesQuery,
    FindCallersQuery, FindEndpointsQuery, FindSymbolQuery, SymbolDefinition,
};

pub struct CsharpAnalyzer;

impl LanguageAnalyzer for CsharpAnalyzer {
    fn language(&self) -> AnalyzerLanguage {
        AnalyzerLanguage::Csharp
    }

    fn supported_extensions(&self) -> &'static [&'static str] {
        &[".cs"]
    }

    fn find_symbols(
        &self,
        path: &Path,
        source: &str,
        query: &FindSymbolQuery,
    ) -> Vec<SymbolDefinition> {
        super::find_symbol::find_symbols(path, source, query)
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
        workspace_root: &Path,
        path: &Path,
        source: &str,
        query: &FindCallersQuery,
    ) -> Vec<CallerDefinition> {
        super::trace_callers::find_callers(workspace_root, path, source, query)
    }

    fn find_callees(
        &self,
        _path: &Path,
        _source: &str,
        _query: &FindCalleesQuery,
    ) -> Vec<CalleeDefinition> {
        Vec::new()
    }

    fn supports_framework(&self, _framework: Option<&str>) -> bool {
        false
    }
}

pub(super) fn node_text(node: Node, source: &[u8]) -> Option<String> {
    node.utf8_text(source)
        .ok()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}
