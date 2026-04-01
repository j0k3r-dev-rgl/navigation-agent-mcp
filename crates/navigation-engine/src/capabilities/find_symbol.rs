use crate::analyzers::{AnalyzerLanguage, AnalyzerRegistry, FindSymbolQuery, SymbolDefinition};
use crate::error::EngineError;
use crate::protocol::{
    EngineRequest, EngineResponse, FindSymbolItem, FindSymbolRequestPayload, FindSymbolResult,
};
use crate::workspace::{
    canonicalize_workspace_root, collect_supported_source_files, contains_hard_ignored_segment,
    public_path, resolve_scope,
};

pub const CAPABILITY: &str = "workspace.find_symbol";

pub fn handle(request: EngineRequest) -> EngineResponse {
    let parsed_payload =
        serde_json::from_value::<FindSymbolRequestPayload>(request.payload.clone());

    match parsed_payload {
        Ok(payload) => match find_symbol(&request.workspace_root, payload) {
            Ok(result) => EngineResponse::success(request.id, &result),
            Err(error) => EngineResponse::error(request.id, error),
        },
        Err(error) => {
            EngineResponse::error(request.id, EngineError::invalid_request(error.to_string()))
        }
    }
}

pub fn find_symbol(
    workspace_root: &str,
    payload: FindSymbolRequestPayload,
) -> Result<FindSymbolResult, EngineError> {
    let workspace_root = canonicalize_workspace_root(workspace_root)?;
    let scope = resolve_scope(&workspace_root, payload.path.as_deref())?;

    if contains_hard_ignored_segment(&workspace_root, &scope.absolute_path) {
        return Ok(FindSymbolResult {
            resolved_path: scope.explicit.then_some(scope.public_path),
            items: Vec::new(),
            total_matched: 0,
            truncated: false,
        });
    }

    let analyzer_language = parse_analyzer_language(&payload.analyzer_language)?;
    let registry = AnalyzerRegistry::new();
    let supported_extensions = registry.supported_extensions(analyzer_language);
    let files =
        collect_supported_source_files(&workspace_root, &scope, &supported_extensions, false)?;
    let query = FindSymbolQuery {
        symbol: payload.symbol,
        kind: payload.kind,
        match_mode: payload.match_mode,
        public_language_filter: payload.public_language_filter,
        limit: payload.limit,
    };

    let mut items = Vec::new();
    for file_path in files {
        let Some(analyzer) = registry.analyzer_for_file(analyzer_language, &file_path) else {
            continue;
        };

        let source = match std::fs::read_to_string(&file_path) {
            Ok(content) => content,
            Err(_) => continue,
        };

        let file_public_path = public_path(&workspace_root, &file_path);
        items.extend(
            analyzer
                .find_symbols(&file_path, &source, &query)
                .into_iter()
                .map(|mut item| {
                    item.path = file_public_path.clone();
                    item
                }),
        );
    }

    let mut filtered = items
        .into_iter()
        .filter(|item| matches_symbol(item, &query))
        .filter(|item| matches_kind(item, &query))
        .filter(|item| matches_public_language(item, &query))
        .collect::<Vec<_>>();

    filtered.sort_by(|left, right| {
        (&left.path, left.line, &left.symbol, &left.kind).cmp(&(
            &right.path,
            right.line,
            &right.symbol,
            &right.kind,
        ))
    });

    let total_matched = filtered.len();
    let truncated = total_matched > query.limit;
    if truncated {
        filtered.truncate(query.limit);
    }

    Ok(FindSymbolResult {
        resolved_path: scope.explicit.then_some(scope.public_path),
        total_matched,
        truncated,
        items: filtered.into_iter().map(map_symbol_item).collect(),
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

fn map_symbol_item(item: SymbolDefinition) -> FindSymbolItem {
    FindSymbolItem {
        symbol: item.symbol,
        kind: item.kind,
        path: item.path,
        line: item.line,
        line_end: item.line_end,
        language: item.language,
    }
}

fn matches_symbol(item: &SymbolDefinition, query: &FindSymbolQuery) -> bool {
    match query.match_mode.as_str() {
        "fuzzy" => item
            .symbol
            .to_ascii_lowercase()
            .contains(&query.symbol.to_ascii_lowercase()),
        _ => item.symbol == query.symbol,
    }
}

fn matches_kind(item: &SymbolDefinition, query: &FindSymbolQuery) -> bool {
    query.kind == "any" || item.kind == query.kind
}

fn matches_public_language(item: &SymbolDefinition, query: &FindSymbolQuery) -> bool {
    match query.public_language_filter.as_deref() {
        Some(expected) => item.language.as_deref() == Some(expected),
        None => true,
    }
}
