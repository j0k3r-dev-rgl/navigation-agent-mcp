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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_symbol_request_payload_uses_camel_case_keys() {
        let payload = FindSymbolRequestPayload {
            symbol: "loader".to_string(),
            path: Some("src/routes".to_string()),
            analyzer_language: "typescript".to_string(),
            public_language_filter: Some("javascript".to_string()),
            kind: "function".to_string(),
            match_mode: "fuzzy".to_string(),
            limit: 25,
        };

        assert_eq!(
            serde_json::to_value(payload).unwrap(),
            serde_json::json!({
                "symbol": "loader",
                "path": "src/routes",
                "analyzerLanguage": "typescript",
                "publicLanguageFilter": "javascript",
                "kind": "function",
                "matchMode": "fuzzy",
                "limit": 25,
            })
        );
    }

    #[test]
    fn find_symbol_request_payload_preserves_python_analyzer_language() {
        let payload = FindSymbolRequestPayload {
            symbol: "fetch_users".to_string(),
            path: Some("profiles".to_string()),
            analyzer_language: "python".to_string(),
            public_language_filter: Some("python".to_string()),
            kind: "function".to_string(),
            match_mode: "exact".to_string(),
            limit: 10,
        };

        assert_eq!(
            serde_json::to_value(payload).unwrap(),
            serde_json::json!({
                "symbol": "fetch_users",
                "path": "profiles",
                "analyzerLanguage": "python",
                "publicLanguageFilter": "python",
                "kind": "function",
                "matchMode": "exact",
                "limit": 10,
            })
        );
    }

    #[test]
    fn find_symbol_request_payload_preserves_rust_analyzer_language() {
        let payload = FindSymbolRequestPayload {
            symbol: "AnalyzerRegistry".to_string(),
            path: Some("crates/navigation-engine/src".to_string()),
            analyzer_language: "rust".to_string(),
            public_language_filter: Some("rust".to_string()),
            kind: "type".to_string(),
            match_mode: "exact".to_string(),
            limit: 10,
        };

        assert_eq!(
            serde_json::to_value(payload).unwrap(),
            serde_json::json!({
                "symbol": "AnalyzerRegistry",
                "path": "crates/navigation-engine/src",
                "analyzerLanguage": "rust",
                "publicLanguageFilter": "rust",
                "kind": "type",
                "matchMode": "exact",
                "limit": 10,
            })
        );
    }

    #[test]
    fn find_symbol_result_uses_camel_case_keys() {
        let result = FindSymbolResult {
            resolved_path: Some("src/routes".to_string()),
            items: vec![FindSymbolItem {
                symbol: "loader".to_string(),
                kind: "function".to_string(),
                path: "src/routes/example.ts".to_string(),
                line: 12,
                language: Some("typescript".to_string()),
            }],
            total_matched: 1,
            truncated: false,
        };

        assert_eq!(
            serde_json::to_value(result).unwrap(),
            serde_json::json!({
                "resolvedPath": "src/routes",
                "items": [{
                    "symbol": "loader",
                    "kind": "function",
                    "path": "src/routes/example.ts",
                    "line": 12,
                    "language": "typescript",
                }],
                "totalMatched": 1,
                "truncated": false,
            })
        );
    }
}
