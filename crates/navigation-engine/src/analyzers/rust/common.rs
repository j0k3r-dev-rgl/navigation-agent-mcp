use std::path::Path;

use tree_sitter::Node;

use super::super::language_analyzer::LanguageAnalyzer;
use super::super::types::{
    AnalyzerLanguage, CalleeDefinition, EndpointDefinition, FindCalleesQuery, FindEndpointsQuery,
    FindSymbolQuery, SymbolDefinition,
};

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

pub(super) fn impl_body(node: Node) -> Option<Node> {
    node.child_by_field_name("body").or_else(|| {
        (0..node.named_child_count())
            .filter_map(|index| node.named_child(index))
            .find(|child| child.kind() == "declaration_list")
    })
}
