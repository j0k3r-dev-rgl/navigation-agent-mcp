use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::EngineError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineRequest {
    pub id: String,
    pub capability: String,
    pub workspace_root: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectTreeRequestPayload {
    pub path: Option<String>,
    pub max_depth: u32,
    pub extensions: Vec<String>,
    pub file_pattern: Option<String>,
    pub include_stats: bool,
    pub include_hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectTreeItemStats {
    pub size_bytes: u64,
    pub modified_at: String,
    pub is_symlink: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectTreeItem {
    pub path: String,
    pub name: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub depth: u32,
    pub extension: Option<String>,
    pub stats: Option<InspectTreeItemStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectTreeResult {
    pub root: String,
    pub items: Vec<InspectTreeItem>,
    pub truncated: bool,
    pub max_items: usize,
    pub ignored_directories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FindSymbolRequestPayload {
    pub symbol: String,
    pub path: Option<String>,
    pub analyzer_language: String,
    pub public_language_filter: Option<String>,
    pub kind: String,
    pub match_mode: String,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FindSymbolItem {
    pub symbol: String,
    pub kind: String,
    pub path: String,
    pub line: u32,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FindSymbolResult {
    pub resolved_path: Option<String>,
    pub items: Vec<FindSymbolItem>,
    pub total_matched: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListEndpointsRequestPayload {
    pub path: Option<String>,
    pub analyzer_language: String,
    pub public_language_filter: Option<String>,
    pub public_framework_filter: Option<String>,
    pub kind: String,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListEndpointsItem {
    pub name: String,
    pub kind: String,
    pub path: Option<String>,
    pub file: String,
    pub line: u32,
    pub language: Option<String>,
    pub framework: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListEndpointsCounts {
    pub by_kind: std::collections::HashMap<String, usize>,
    pub by_language: std::collections::HashMap<String, usize>,
    pub by_framework: std::collections::HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListEndpointsResult {
    pub resolved_path: Option<String>,
    pub items: Vec<ListEndpointsItem>,
    pub total_matched: usize,
    pub truncated: bool,
    pub counts: ListEndpointsCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineSuccess {
    pub id: String,
    pub ok: bool,
    pub result: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineFailure {
    pub id: String,
    pub ok: bool,
    pub error: EngineError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EngineResponse {
    Success(EngineSuccess),
    Failure(EngineFailure),
}

impl EngineResponse {
    pub fn success<T>(id: String, result: &T) -> Self
    where
        T: Serialize,
    {
        let result = serde_json::to_value(result).unwrap_or(Value::Null);
        Self::Success(EngineSuccess {
            id,
            ok: true,
            result,
        })
    }

    pub fn error(id: String, error: EngineError) -> Self {
        Self::Failure(EngineFailure {
            id,
            ok: false,
            error,
        })
    }
}
