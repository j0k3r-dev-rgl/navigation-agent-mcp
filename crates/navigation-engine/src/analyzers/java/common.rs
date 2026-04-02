use std::path::Path;

use tree_sitter::Node;

use super::super::language_analyzer::LanguageAnalyzer;
use super::super::types::{
    AnalyzerLanguage, CalleeDefinition, CallerDefinition, EndpointDefinition, FindCalleesQuery,
    FindCallersQuery, FindEndpointsQuery, FindSymbolQuery, SymbolDefinition,
};

pub struct JavaAnalyzer;

impl LanguageAnalyzer for JavaAnalyzer {
    fn language(&self) -> AnalyzerLanguage {
        AnalyzerLanguage::Java
    }

    fn supported_extensions(&self) -> &'static [&'static str] {
        &[".java"]
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
        path: &Path,
        source: &str,
        query: &FindCalleesQuery,
    ) -> Vec<CalleeDefinition> {
        super::trace_flow::find_callees(path, source, query)
    }

    fn supports_framework(&self, framework: Option<&str>) -> bool {
        match framework {
            None => true,
            Some("spring") => true,
            Some(_) => false,
        }
    }
}

pub(super) fn node_text(node: Node, source: &[u8]) -> Option<String> {
    node.utf8_text(source)
        .ok()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub(super) fn find_modifiers_child<'a>(node: &Node<'a>) -> Option<Node<'a>> {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "modifiers" {
                return Some(child);
            }
        }
    }
    None
}

pub(super) fn extract_marker_annotation_name(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| node_text(n, source))
}

pub(super) fn extract_annotation_name(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| node_text(n, source))
}
