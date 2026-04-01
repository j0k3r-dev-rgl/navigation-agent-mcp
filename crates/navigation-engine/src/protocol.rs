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
    pub line_end: u32,
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
pub struct SearchTextRequestPayload {
    pub query: String,
    pub path: Option<String>,
    pub public_language_filter: Option<String>,
    pub include: Option<String>,
    pub regex: bool,
    pub context: usize,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SearchTextContextLine {
    pub line: u32,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SearchTextSubmatch {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SearchTextMatch {
    pub line: u32,
    pub text: String,
    pub submatches: Vec<SearchTextSubmatch>,
    pub before: Vec<SearchTextContextLine>,
    pub after: Vec<SearchTextContextLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SearchTextFileMatch {
    pub path: String,
    pub language: Option<String>,
    pub match_count: usize,
    pub matches: Vec<SearchTextMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchTextResult {
    pub resolved_path: Option<String>,
    pub items: Vec<SearchTextFileMatch>,
    pub total_file_count: usize,
    pub total_match_count: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceFlowRequestPayload {
    pub path: String,
    pub symbol: String,
    pub analyzer_language: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_language_filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_depth: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct TraceSymbolItem {
    pub path: String,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceFlowResult {
    pub resolved_path: Option<String>,
    pub items: Vec<TraceSymbolItem>,
    pub total_matched: usize,
    pub truncated: bool,
    pub callees: Vec<CalleeItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalleeItem {
    pub path: String,
    pub line: u32,
    pub end_line: u32,
    pub column: Option<u32>,
    pub callee: String,
    pub callee_symbol: Option<String>,
    pub relation: String,
    pub language: Option<String>,
    pub snippet: Option<String>,
    pub depth: u32,
    pub call_chain: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceCallersRequestPayload {
    pub path: String,
    pub symbol: String,
    pub analyzer_language: String,
    pub public_language_filter: Option<String>,
    pub recursive: bool,
    pub max_depth: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraceCallersItem {
    pub path: String,
    pub line: u32,
    pub column: Option<u32>,
    pub caller: String,
    pub caller_symbol: Option<String>,
    pub relation: String,
    pub language: Option<String>,
    pub snippet: Option<String>,
    pub receiver_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraceCallersRecursiveVia {
    pub relation: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraceCallersRecursiveNode {
    pub key: String,
    pub path: String,
    pub symbol: String,
    pub depth: u32,
    pub via: Option<TraceCallersRecursiveVia>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraceCallersRecursivePathSegment {
    pub path: String,
    pub symbol: String,
    pub depth: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraceCallersRecursiveCycle {
    pub from_key: String,
    pub to_key: String,
    pub path: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraceCallersRecursiveTruncatedNode {
    pub key: String,
    pub path: String,
    pub symbol: String,
    pub depth: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraceCallersProbableEntryPoint {
    pub key: Option<String>,
    pub path: String,
    pub symbol: String,
    pub depth: Option<u32>,
    pub reasons: Vec<String>,
    pub probable: Option<bool>,
    pub path_from_target: Vec<TraceCallersRecursivePathSegment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraceCallersCallsTarget {
    pub path: String,
    pub symbol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraceCallersClassificationRecord {
    pub path: String,
    pub symbol: String,
    pub caller: String,
    pub depth: u32,
    pub line: u32,
    pub column: Option<u32>,
    pub relation: String,
    pub language: Option<String>,
    pub receiver_type: Option<String>,
    pub snippet: Option<String>,
    pub calls: TraceCallersCallsTarget,
    pub path_from_target: Vec<TraceCallersRecursivePathSegment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraceCallersImplementationInterface {
    pub name: Option<String>,
    pub path: Option<String>,
    pub symbol: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraceCallersImplementationReference {
    pub path: String,
    pub symbol: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraceCallersImplementationInterfaceChain {
    pub kind: String,
    pub probable: Option<bool>,
    pub interface: Option<TraceCallersImplementationInterface>,
    pub implementation: Option<TraceCallersImplementationReference>,
    pub implementations: Vec<TraceCallersImplementationReference>,
    pub callers: Vec<TraceCallersClassificationRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraceCallersRecursiveSummary {
    pub direct_caller_count: usize,
    pub indirect_caller_count: usize,
    pub probable_public_entry_point_count: usize,
    pub implementation_interface_chain_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraceCallersRecursiveClassifications {
    pub summary: TraceCallersRecursiveSummary,
    pub direct_callers: Vec<TraceCallersClassificationRecord>,
    pub indirect_callers: Vec<TraceCallersClassificationRecord>,
    pub probable_public_entry_points: Vec<TraceCallersProbableEntryPoint>,
    pub implementation_interface_chain: Vec<TraceCallersImplementationInterfaceChain>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraceCallersRecursiveResult {
    pub enabled: bool,
    pub root: TraceCallersRecursiveNode,
    pub max_depth: u32,
    pub max_depth_observed: u32,
    pub node_count: usize,
    pub edge_count: usize,
    pub path_count: usize,
    pub nodes: Vec<TraceCallersRecursiveNode>,
    pub adjacency: std::collections::BTreeMap<String, Vec<String>>,
    pub paths: Vec<Vec<TraceCallersRecursivePathSegment>>,
    pub cycles: Vec<TraceCallersRecursiveCycle>,
    pub truncated: Vec<TraceCallersRecursiveTruncatedNode>,
    pub probable_entry_points: Vec<TraceCallersProbableEntryPoint>,
    pub classifications: TraceCallersRecursiveClassifications,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceCallersResult {
    pub resolved_path: Option<String>,
    pub items: Vec<TraceCallersItem>,
    pub total_matched: usize,
    pub truncated: bool,
    pub recursive: Option<TraceCallersRecursiveResult>,
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
