use navigation_engine::protocol::{
    FindSymbolItem, FindSymbolRequestPayload, FindSymbolResult, SearchTextContextLine,
    SearchTextFileMatch, SearchTextMatch, SearchTextRequestPayload, SearchTextResult,
    SearchTextSubmatch, TraceCallersItem, TraceCallersRequestPayload, TraceCallersResult,
    TraceSymbolItem, TraceSymbolRequestPayload, TraceSymbolResult,
};

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

#[test]
fn search_text_request_payload_uses_camel_case_keys() {
    let payload = SearchTextRequestPayload {
        query: "loader".to_string(),
        path: Some("src/routes".to_string()),
        public_language_filter: Some("typescript".to_string()),
        include: Some("src/**".to_string()),
        regex: false,
        context: 2,
        limit: 25,
    };

    assert_eq!(
        serde_json::to_value(payload).unwrap(),
        serde_json::json!({
            "query": "loader",
            "path": "src/routes",
            "publicLanguageFilter": "typescript",
            "include": "src/**",
            "regex": false,
            "context": 2,
            "limit": 25,
        })
    );
}

#[test]
fn search_text_result_uses_camel_case_keys() {
    let result = SearchTextResult {
        resolved_path: Some("src/routes".to_string()),
        items: vec![SearchTextFileMatch {
            path: "src/routes/example.ts".to_string(),
            language: Some("typescript".to_string()),
            match_count: 1,
            matches: vec![SearchTextMatch {
                line: 12,
                text: "export async function loader() {}".to_string(),
                submatches: vec![SearchTextSubmatch {
                    start: 22,
                    end: 28,
                    text: "loader".to_string(),
                }],
                before: vec![SearchTextContextLine {
                    line: 11,
                    text: "const before = true;".to_string(),
                }],
                after: vec![SearchTextContextLine {
                    line: 13,
                    text: "return null;".to_string(),
                }],
            }],
        }],
        total_file_count: 1,
        total_match_count: 1,
        truncated: false,
    };

    assert_eq!(
        serde_json::to_value(result).unwrap(),
        serde_json::json!({
            "resolvedPath": "src/routes",
            "items": [{
                "path": "src/routes/example.ts",
                "language": "typescript",
                "matchCount": 1,
                "matches": [{
                    "line": 12,
                    "text": "export async function loader() {}",
                    "submatches": [{
                        "start": 22,
                        "end": 28,
                        "text": "loader",
                    }],
                    "before": [{
                        "line": 11,
                        "text": "const before = true;",
                    }],
                    "after": [{
                        "line": 13,
                        "text": "return null;",
                    }],
                }],
            }],
            "totalFileCount": 1,
            "totalMatchCount": 1,
            "truncated": false,
        })
    );
}

#[test]
fn trace_symbol_request_payload_uses_camel_case_keys() {
    let payload = TraceSymbolRequestPayload {
        path: "src/routes/dashboard.tsx".to_string(),
        symbol: "loader".to_string(),
        analyzer_language: "typescript".to_string(),
        public_language_filter: Some("typescript".to_string()),
    };

    assert_eq!(
        serde_json::to_value(payload).unwrap(),
        serde_json::json!({
            "path": "src/routes/dashboard.tsx",
            "symbol": "loader",
            "analyzerLanguage": "typescript",
            "publicLanguageFilter": "typescript",
        })
    );
}

#[test]
fn trace_symbol_result_uses_camel_case_keys() {
    let result = TraceSymbolResult {
        resolved_path: Some("src/routes/dashboard.tsx".to_string()),
        items: vec![TraceSymbolItem {
            path: "src/shared/navigation.ts".to_string(),
            language: Some("typescript".to_string()),
        }],
        total_matched: 1,
        truncated: false,
    };

    assert_eq!(
        serde_json::to_value(result).unwrap(),
        serde_json::json!({
            "resolvedPath": "src/routes/dashboard.tsx",
            "items": [{
                "path": "src/shared/navigation.ts",
                "language": "typescript",
            }],
            "totalMatched": 1,
            "truncated": false,
        })
    );
}

#[test]
fn trace_callers_request_payload_uses_camel_case_keys() {
    let payload = TraceCallersRequestPayload {
        path: "src/routes/dashboard.tsx".to_string(),
        symbol: "loader".to_string(),
        analyzer_language: "typescript".to_string(),
        public_language_filter: Some("typescript".to_string()),
        recursive: true,
        max_depth: Some(4),
    };

    assert_eq!(
        serde_json::to_value(payload).unwrap(),
        serde_json::json!({
            "path": "src/routes/dashboard.tsx",
            "symbol": "loader",
            "analyzerLanguage": "typescript",
            "publicLanguageFilter": "typescript",
            "recursive": true,
            "maxDepth": 4,
        })
    );
}

#[test]
fn trace_callers_result_uses_camel_case_keys() {
    let result = TraceCallersResult {
        resolved_path: Some("src/routes/dashboard.tsx".to_string()),
        items: vec![TraceCallersItem {
            path: "src/routes/layout.tsx".to_string(),
            line: 9,
            column: Some(3),
            caller: "Layout".to_string(),
            caller_symbol: Some("Layout".to_string()),
            relation: "calls".to_string(),
            language: Some("typescript".to_string()),
            snippet: Some("loader()".to_string()),
            receiver_type: None,
        }],
        total_matched: 1,
        truncated: false,
        recursive: None,
    };

    assert_eq!(
        serde_json::to_value(result).unwrap(),
        serde_json::json!({
            "resolvedPath": "src/routes/dashboard.tsx",
            "items": [{
                "path": "src/routes/layout.tsx",
                "line": 9,
                "column": 3,
                "caller": "Layout",
                "callerSymbol": "Layout",
                "relation": "calls",
                "language": "typescript",
                "snippet": "loader()",
                "receiverType": null,
            }],
            "totalMatched": 1,
            "truncated": false,
            "recursive": null,
        })
    );
}
