use std::path::Path;

use tree_sitter::Node;

use super::super::language_analyzer::LanguageAnalyzer;
use super::super::types::{
    AnalyzerLanguage, CalleeDefinition, CallerDefinition, EndpointDefinition, FindCalleesQuery,
    FindCallersQuery, FindEndpointsQuery, FindSymbolQuery, SymbolDefinition,
};

pub struct PythonAnalyzer;

impl LanguageAnalyzer for PythonAnalyzer {
    fn language(&self) -> AnalyzerLanguage {
        AnalyzerLanguage::Python
    }

    fn supported_extensions(&self) -> &'static [&'static str] {
        &[".py"]
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
        path: &Path,
        source: &str,
        query: &FindEndpointsQuery,
    ) -> Vec<EndpointDefinition> {
        super::find_endpoints::find_endpoints(path, source, query)
    }

    fn find_callees(
        &self,
        path: &Path,
        source: &str,
        query: &FindCalleesQuery,
    ) -> Vec<CalleeDefinition> {
        super::trace_flow::find_callees(path, source, query)
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

    fn supports_framework(&self, framework: Option<&str>) -> bool {
        framework.is_none()
    }
}

pub(super) fn node_text(node: Node, source: &[u8]) -> Option<String> {
    node.utf8_text(source)
        .ok()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}
