use std::path::{Path, PathBuf};

use tree_sitter::Node;

use super::super::language_analyzer::LanguageAnalyzer;
use super::super::types::{
    AnalyzerLanguage, CalleeDefinition, CallerDefinition, EndpointDefinition, FindCalleesQuery,
    FindCallersQuery, FindEndpointsQuery, FindSymbolQuery, SymbolDefinition,
};

pub struct TypeScriptAnalyzer;

impl LanguageAnalyzer for TypeScriptAnalyzer {
    fn language(&self) -> AnalyzerLanguage {
        AnalyzerLanguage::Typescript
    }

    fn supported_extensions(&self) -> &'static [&'static str] {
        &[".ts", ".tsx", ".js", ".jsx"]
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
            Some("react-router") => true,
            Some(_) => false,
        }
    }
}

pub(super) fn parser_language_for_path(path: &Path) -> Option<tree_sitter_language::LanguageFn> {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
    {
        Some(extension) if extension == "ts" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT),
        Some(extension) if extension == "tsx" => Some(tree_sitter_typescript::LANGUAGE_TSX),
        Some(extension) if extension == "js" || extension == "jsx" => {
            Some(tree_sitter_javascript::LANGUAGE)
        }
        _ => None,
    }
}

pub(super) fn find_workspace_root(start_path: &Path) -> PathBuf {
    let mut current = start_path.parent();
    while let Some(dir) = current {
        if dir.join("package.json").exists() || dir.join("tsconfig.json").exists() {
            return dir.to_path_buf();
        }
        current = dir.parent();
    }

    start_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

pub(super) fn node_text(node: Node, source: &[u8]) -> Option<String> {
    node.utf8_text(source)
        .ok()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub(super) fn derive_route_path_from_file(path: &Path) -> Option<String> {
    let path_str = path.to_string_lossy();
    let routes_idx = path_str.find("/routes/")?;
    let route_file = &path_str[routes_idx + 8..];
    let route_name = route_file.rsplit_once('.')?.0;

    if route_name == "_index" || route_name.ends_with("/_index") {
        let parent = route_name.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
        if parent.is_empty() {
            return Some("/".to_string());
        }
        return Some(format!("/{}", parent.replace('.', "/")));
    }

    let segments: Vec<&str> = route_name
        .split('/')
        .last()
        .unwrap_or(route_name)
        .split('.')
        .collect();

    let path_segments: Vec<String> = segments
        .iter()
        .filter(|s| !s.starts_with('_'))
        .map(|s| {
            if s.starts_with('$') {
                format!(":{}", &s[1..])
            } else {
                s.to_string()
            }
        })
        .collect();

    if path_segments.is_empty() {
        return Some("/".to_string());
    }

    Some(format!("/{}", path_segments.join("/")))
}
