use std::collections::BTreeSet;

use crate::error::EngineError;
use crate::protocol::{
    EngineRequest, EngineResponse, FindSymbolRequestPayload, SearchTextRequestPayload,
    TraceSymbolItem, TraceSymbolRequestPayload, TraceSymbolResult,
};
use crate::workspace::{canonicalize_workspace_root, contains_hard_ignored_segment, resolve_scope};

use super::find_symbol::find_symbol;
use super::search_text::search_text;

pub const CAPABILITY: &str = "workspace.trace_symbol";

pub fn handle(request: EngineRequest) -> EngineResponse {
    let parsed_payload =
        serde_json::from_value::<TraceSymbolRequestPayload>(request.payload.clone());

    match parsed_payload {
        Ok(payload) => match trace_symbol(&request.workspace_root, payload) {
            Ok(result) => EngineResponse::success(request.id, &result),
            Err(error) => EngineResponse::error(request.id, error),
        },
        Err(error) => {
            EngineResponse::error(request.id, EngineError::invalid_request(error.to_string()))
        }
    }
}

pub fn trace_symbol(
    workspace_root: &str,
    payload: TraceSymbolRequestPayload,
) -> Result<TraceSymbolResult, EngineError> {
    let workspace_root = canonicalize_workspace_root(workspace_root)?;
    let scope = resolve_scope(&workspace_root, Some(payload.path.as_str()))?;

    if !scope.absolute_path.is_file() {
        return Err(EngineError::file_not_found(payload.path.as_str()));
    }

    if contains_hard_ignored_segment(&workspace_root, &scope.absolute_path) {
        return Ok(TraceSymbolResult {
            resolved_path: Some(scope.public_path),
            items: Vec::new(),
            total_matched: 0,
            truncated: false,
        });
    }

    let start_file_path = payload.path.clone();
    let symbol_check = find_symbol(
        workspace_root.to_string_lossy().as_ref(),
        FindSymbolRequestPayload {
            symbol: payload.symbol.clone(),
            path: Some(start_file_path.clone()),
            analyzer_language: payload.analyzer_language.clone(),
            public_language_filter: payload.public_language_filter.clone(),
            kind: "any".to_string(),
            match_mode: "exact".to_string(),
            limit: usize::MAX,
        },
    )?;

    if symbol_check.total_matched == 0 {
        return Err(EngineError::symbol_not_found(
            payload.symbol.as_str(),
            start_file_path.as_str(),
        ));
    }

    let search_result = search_text(
        workspace_root.to_string_lossy().as_ref(),
        SearchTextRequestPayload {
            query: payload.symbol,
            path: None,
            public_language_filter: payload.public_language_filter,
            include: None,
            regex: false,
            context: 0,
            limit: usize::MAX,
        },
    )?;

    let items = search_result
        .items
        .into_iter()
        .map(|item| TraceSymbolItem {
            path: item.path,
            language: item.language,
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    Ok(TraceSymbolResult {
        resolved_path: Some(scope.public_path),
        total_matched: items.len(),
        truncated: false,
        items,
    })
}
