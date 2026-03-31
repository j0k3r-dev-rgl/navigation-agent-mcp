use std::collections::HashMap;

use crate::analyzers::{
    AnalyzerLanguage, AnalyzerRegistry, EndpointDefinition, FindEndpointsQuery,
};
use crate::error::EngineError;
use crate::protocol::{
    EngineRequest, EngineResponse, ListEndpointsCounts, ListEndpointsItem,
    ListEndpointsRequestPayload, ListEndpointsResult,
};
use crate::workspace::{
    canonicalize_workspace_root, collect_supported_source_files, contains_hard_ignored_segment,
    public_path, resolve_scope,
};

pub const CAPABILITY: &str = "workspace.list_endpoints";

pub fn handle(request: EngineRequest) -> EngineResponse {
    let parsed_payload =
        serde_json::from_value::<ListEndpointsRequestPayload>(request.payload.clone());

    match parsed_payload {
        Ok(payload) => match list_endpoints(&request.workspace_root, payload) {
            Ok(result) => EngineResponse::success(request.id, &result),
            Err(error) => EngineResponse::error(request.id, error),
        },
        Err(error) => {
            EngineResponse::error(request.id, EngineError::invalid_request(error.to_string()))
        }
    }
}

pub fn list_endpoints(
    workspace_root: &str,
    payload: ListEndpointsRequestPayload,
) -> Result<ListEndpointsResult, EngineError> {
    let workspace_root = canonicalize_workspace_root(workspace_root)?;
    let scope = resolve_scope(&workspace_root, payload.path.as_deref())?;

    if contains_hard_ignored_segment(&workspace_root, &scope.absolute_path) {
        return Ok(ListEndpointsResult {
            resolved_path: scope.explicit.then_some(scope.public_path),
            items: Vec::new(),
            total_matched: 0,
            truncated: false,
            counts: ListEndpointsCounts {
                by_kind: HashMap::new(),
                by_language: HashMap::new(),
                by_framework: HashMap::new(),
            },
        });
    }

    let analyzer_language = parse_analyzer_language(&payload.analyzer_language)?;
    let registry = AnalyzerRegistry::new();
    let supported_extensions = registry.supported_extensions(analyzer_language);
    let files =
        collect_supported_source_files(&workspace_root, &scope, &supported_extensions, false)?;
    let query = FindEndpointsQuery {
        kind: payload.kind,
        public_language_filter: payload.public_language_filter,
        public_framework_filter: payload.public_framework_filter,
        limit: payload.limit,
    };

    let mut items = Vec::new();
    for file_path in files {
        let Some(analyzer) = registry.analyzer_for_file(analyzer_language, &file_path) else {
            continue;
        };

        // Skip if framework filter doesn't match analyzer's supported frameworks
        if !analyzer.supports_framework(query.public_framework_filter.as_deref()) {
            continue;
        }

        let source = std::fs::read_to_string(&file_path)
            .map_err(|error| EngineError::backend_execution_failed(error.to_string()))?;

        let file_public_path = public_path(&workspace_root, &file_path);
        items.extend(
            analyzer
                .find_endpoints(&file_path, &source, &query)
                .into_iter()
                .map(|mut item| {
                    item.file = file_public_path.clone();
                    item
                }),
        );
    }

    let mut filtered = items
        .into_iter()
        .filter(|item| matches_kind(item, &query))
        .filter(|item| matches_public_language(item, &query))
        .filter(|item| matches_public_framework(item, &query))
        .collect::<Vec<_>>();

    filtered.sort_by(|left, right| {
        (&left.kind, &left.path, &left.file, left.line, &left.name).cmp(&(
            &right.kind,
            &right.path,
            &right.file,
            right.line,
            &right.name,
        ))
    });

    let total_matched = filtered.len();
    let truncated = total_matched > query.limit;
    if truncated {
        filtered.truncate(query.limit);
    }

    let items: Vec<ListEndpointsItem> = filtered.into_iter().map(map_endpoint_item).collect();
    let counts = calculate_counts(&items);

    Ok(ListEndpointsResult {
        resolved_path: scope.explicit.then_some(scope.public_path),
        total_matched,
        truncated,
        items,
        counts,
    })
}

fn parse_analyzer_language(value: &str) -> Result<AnalyzerLanguage, EngineError> {
    match value {
        "auto" => Ok(AnalyzerLanguage::Auto),
        "java" => Ok(AnalyzerLanguage::Java),
        "python" => Ok(AnalyzerLanguage::Python),
        "rust" => Ok(AnalyzerLanguage::Rust),
        "typescript" => Ok(AnalyzerLanguage::Typescript),
        other => Err(EngineError::invalid_request(format!(
            "Unsupported analyzerLanguage '{}'.",
            other
        ))),
    }
}

fn map_endpoint_item(item: EndpointDefinition) -> ListEndpointsItem {
    ListEndpointsItem {
        name: item.name,
        kind: item.kind,
        path: item.path,
        file: item.file,
        line: item.line,
        language: item.language,
        framework: item.framework,
    }
}

fn matches_kind(item: &EndpointDefinition, query: &FindEndpointsQuery) -> bool {
    query.kind == "any" || item.kind == query.kind
}

fn matches_public_language(item: &EndpointDefinition, query: &FindEndpointsQuery) -> bool {
    match query.public_language_filter.as_deref() {
        Some(expected) => item.language.as_deref() == Some(expected),
        None => true,
    }
}

fn matches_public_framework(item: &EndpointDefinition, query: &FindEndpointsQuery) -> bool {
    match query.public_framework_filter.as_deref() {
        Some(expected) => item.framework.as_deref() == Some(expected),
        None => true,
    }
}

fn calculate_counts(items: &[ListEndpointsItem]) -> ListEndpointsCounts {
    let mut by_kind: HashMap<String, usize> = HashMap::new();
    let mut by_language: HashMap<String, usize> = HashMap::new();
    let mut by_framework: HashMap<String, usize> = HashMap::new();

    for item in items {
        *by_kind.entry(item.kind.clone()).or_insert(0) += 1;
        if let Some(ref lang) = item.language {
            *by_language.entry(lang.clone()).or_insert(0) += 1;
        }
        if let Some(ref fw) = item.framework {
            *by_framework.entry(fw.clone()).or_insert(0) += 1;
        }
    }

    ListEndpointsCounts {
        by_kind,
        by_language,
        by_framework,
    }
}
