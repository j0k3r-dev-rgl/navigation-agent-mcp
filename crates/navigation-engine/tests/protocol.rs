use navigation_engine::protocol::{FindSymbolItem, FindSymbolRequestPayload, FindSymbolResult};

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
