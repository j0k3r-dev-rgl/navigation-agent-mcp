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

fn find_symbol(
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

        let source = std::fs::read_to_string(&file_path)
            .map_err(|error| EngineError::backend_execution_failed(error.to_string()))?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn returns_real_typescript_results_with_exact_match() {
        let workspace = tempdir().unwrap();
        std::fs::create_dir_all(workspace.path().join("src")).unwrap();
        std::fs::write(
            workspace.path().join("src/example.ts"),
            "export function loader() {}\nexport function load() {}\n",
        )
        .unwrap();

        let result = find_symbol(
            workspace.path().to_string_lossy().as_ref(),
            FindSymbolRequestPayload {
                symbol: "loader".to_string(),
                path: Some("src".to_string()),
                analyzer_language: "typescript".to_string(),
                public_language_filter: Some("typescript".to_string()),
                kind: "any".to_string(),
                match_mode: "exact".to_string(),
                limit: 50,
            },
        )
        .unwrap();

        assert_eq!(result.resolved_path.as_deref(), Some("src"));
        assert_eq!(result.total_matched, 1);
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].symbol, "loader");
        assert_eq!(result.items[0].kind, "function");
        assert_eq!(result.items[0].path, "src/example.ts");
        assert_eq!(result.items[0].language.as_deref(), Some("typescript"));
        assert!(!result.truncated);
    }

    #[test]
    fn supports_fuzzy_matching_javascript_filter_and_truncation() {
        let workspace = tempdir().unwrap();
        std::fs::create_dir_all(workspace.path().join("src")).unwrap();
        std::fs::write(
            workspace.path().join("src/example.ts"),
            "export function loader() {}\nexport const loadAction = () => {}\n",
        )
        .unwrap();
        std::fs::write(
            workspace.path().join("src/example.js"),
            "export const loaderJs = () => {}\n",
        )
        .unwrap();

        let result = find_symbol(
            workspace.path().to_string_lossy().as_ref(),
            FindSymbolRequestPayload {
                symbol: "load".to_string(),
                path: Some("src".to_string()),
                analyzer_language: "typescript".to_string(),
                public_language_filter: Some("javascript".to_string()),
                kind: "any".to_string(),
                match_mode: "fuzzy".to_string(),
                limit: 1,
            },
        )
        .unwrap();

        assert_eq!(result.total_matched, 1);
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].symbol, "loaderJs");
        assert_eq!(result.items[0].language.as_deref(), Some("javascript"));
        assert!(!result.truncated);

        let truncated_result = find_symbol(
            workspace.path().to_string_lossy().as_ref(),
            FindSymbolRequestPayload {
                symbol: "load".to_string(),
                path: Some("src".to_string()),
                analyzer_language: "typescript".to_string(),
                public_language_filter: None,
                kind: "function".to_string(),
                match_mode: "fuzzy".to_string(),
                limit: 2,
            },
        )
        .unwrap();

        assert_eq!(truncated_result.total_matched, 3);
        assert_eq!(truncated_result.items.len(), 2);
        assert!(truncated_result.truncated);
        let ordered = truncated_result
            .items
            .iter()
            .map(|item| {
                (
                    item.path.as_str(),
                    item.line,
                    item.symbol.as_str(),
                    item.kind.as_str(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            ordered,
            vec![
                ("src/example.js", 1, "loaderJs", "function"),
                ("src/example.ts", 1, "loader", "function"),
            ]
        );
    }

    #[test]
    fn filters_results_by_requested_kind() {
        let workspace = tempdir().unwrap();
        std::fs::create_dir_all(workspace.path().join("src")).unwrap();
        std::fs::write(
            workspace.path().join("src/example.ts"),
            "class Loader { loader() {} }\nfunction loader() {}\n",
        )
        .unwrap();

        let result = find_symbol(
            workspace.path().to_string_lossy().as_ref(),
            FindSymbolRequestPayload {
                symbol: "loader".to_string(),
                path: Some("src".to_string()),
                analyzer_language: "typescript".to_string(),
                public_language_filter: None,
                kind: "method".to_string(),
                match_mode: "exact".to_string(),
                limit: 50,
            },
        )
        .unwrap();

        assert_eq!(result.total_matched, 1);
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].kind, "method");
        assert_eq!(result.items[0].symbol, "loader");
    }

    #[test]
    fn returns_real_java_results_with_language_filter() {
        let workspace = tempdir().unwrap();
        std::fs::create_dir_all(workspace.path().join("src/main/java/demo")).unwrap();
        std::fs::write(
            workspace
                .path()
                .join("src/main/java/demo/ExampleService.java"),
            "public class ExampleService { public ExampleService() {} public void execute() {} }\n",
        )
        .unwrap();

        let result = find_symbol(
            workspace.path().to_string_lossy().as_ref(),
            FindSymbolRequestPayload {
                symbol: "ExampleService".to_string(),
                path: Some("src/main/java".to_string()),
                analyzer_language: "java".to_string(),
                public_language_filter: Some("java".to_string()),
                kind: "any".to_string(),
                match_mode: "exact".to_string(),
                limit: 50,
            },
        )
        .unwrap();

        assert_eq!(result.resolved_path.as_deref(), Some("src/main/java"));
        assert_eq!(result.total_matched, 2);
        assert_eq!(result.items.len(), 2);
        assert_eq!(
            result.items[0].path,
            "src/main/java/demo/ExampleService.java"
        );
        assert_eq!(result.items[0].kind, "class");
        assert_eq!(result.items[0].language.as_deref(), Some("java"));
        assert_eq!(result.items[1].kind, "constructor");
    }

    #[test]
    fn returns_python_exact_class_matches_only() {
        let workspace = tempdir().unwrap();
        std::fs::create_dir_all(workspace.path().join("py")).unwrap();
        std::fs::write(
            workspace.path().join("py/models.py"),
            "class Loader:\n    pass\n\nclass LoaderFactory:\n    pass\n",
        )
        .unwrap();

        let result = find_symbol(
            workspace.path().to_string_lossy().as_ref(),
            FindSymbolRequestPayload {
                symbol: "Loader".to_string(),
                path: Some("py".to_string()),
                analyzer_language: "python".to_string(),
                public_language_filter: Some("python".to_string()),
                kind: "class".to_string(),
                match_mode: "exact".to_string(),
                limit: 50,
            },
        )
        .unwrap();

        assert_eq!(result.resolved_path.as_deref(), Some("py"));
        assert_eq!(result.total_matched, 1);
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].symbol, "Loader");
        assert_eq!(result.items[0].kind, "class");
        assert_eq!(result.items[0].language.as_deref(), Some("python"));
    }

    #[test]
    fn returns_python_exact_method_without_non_exact_functions() {
        let workspace = tempdir().unwrap();
        std::fs::create_dir_all(workspace.path().join("py")).unwrap();
        std::fs::write(
            workspace.path().join("py/service.py"),
            "class Runner:\n    def run(self):\n        return True\n\ndef run_task():\n    return True\n\ndef runner():\n    return True\n",
        )
        .unwrap();

        let result = find_symbol(
            workspace.path().to_string_lossy().as_ref(),
            FindSymbolRequestPayload {
                symbol: "run".to_string(),
                path: Some("py".to_string()),
                analyzer_language: "python".to_string(),
                public_language_filter: Some("python".to_string()),
                kind: "any".to_string(),
                match_mode: "exact".to_string(),
                limit: 50,
            },
        )
        .unwrap();

        assert_eq!(result.total_matched, 1);
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].symbol, "run");
        assert_eq!(result.items[0].kind, "method");
    }

    #[test]
    fn supports_python_fuzzy_matching_kind_path_truncation_and_deduplication() {
        let workspace = tempdir().unwrap();
        std::fs::create_dir_all(workspace.path().join("py/in_scope")).unwrap();
        std::fs::create_dir_all(workspace.path().join("py/out_scope")).unwrap();
        std::fs::write(
            workspace.path().join("py/in_scope/service.py"),
            "def load():\n    return 1\n\ndef load_data():\n    return 2\n\nclass Loader:\n    @staticmethod\n    def load_cached():\n        return 3\n\n@decorator\ndef load_decorated():\n    return 4\n",
        )
        .unwrap();
        std::fs::write(
            workspace.path().join("py/in_scope/async_defs.py"),
            "class Syncer:\n    async def load(self):\n        return 1\n\nasync def load_async():\n    return 2\n\nvalue = lambda: 1\nfrom other import load_alias\n\ndef outer():\n    def load_nested():\n        return 3\n    return load_nested\n",
        )
        .unwrap();
        std::fs::write(
            workspace.path().join("py/out_scope/other.py"),
            "def load_elsewhere():\n    return 0\n",
        )
        .unwrap();

        let fuzzy = find_symbol(
            workspace.path().to_string_lossy().as_ref(),
            FindSymbolRequestPayload {
                symbol: "load".to_string(),
                path: Some("py/in_scope".to_string()),
                analyzer_language: "python".to_string(),
                public_language_filter: Some("python".to_string()),
                kind: "any".to_string(),
                match_mode: "fuzzy".to_string(),
                limit: 50,
            },
        )
        .unwrap();

        let ordered = fuzzy
            .items
            .iter()
            .map(|item| {
                (
                    item.path.as_str(),
                    item.line,
                    item.symbol.as_str(),
                    item.kind.as_str(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            ordered,
            vec![
                ("py/in_scope/async_defs.py", 2, "load", "method"),
                ("py/in_scope/async_defs.py", 5, "load_async", "function"),
                ("py/in_scope/service.py", 1, "load", "function"),
                ("py/in_scope/service.py", 4, "load_data", "function"),
                ("py/in_scope/service.py", 7, "Loader", "class"),
                ("py/in_scope/service.py", 9, "load_cached", "method"),
                ("py/in_scope/service.py", 13, "load_decorated", "function"),
            ]
        );
        assert!(fuzzy
            .items
            .iter()
            .all(|item| item.language.as_deref() == Some("python")));

        let methods_only = find_symbol(
            workspace.path().to_string_lossy().as_ref(),
            FindSymbolRequestPayload {
                symbol: "load".to_string(),
                path: Some("py/in_scope".to_string()),
                analyzer_language: "python".to_string(),
                public_language_filter: Some("python".to_string()),
                kind: "method".to_string(),
                match_mode: "fuzzy".to_string(),
                limit: 50,
            },
        )
        .unwrap();
        assert_eq!(methods_only.total_matched, 2);
        assert!(methods_only.items.iter().all(|item| item.kind == "method"));

        let truncated = find_symbol(
            workspace.path().to_string_lossy().as_ref(),
            FindSymbolRequestPayload {
                symbol: "load".to_string(),
                path: Some("py/in_scope".to_string()),
                analyzer_language: "python".to_string(),
                public_language_filter: Some("python".to_string()),
                kind: "any".to_string(),
                match_mode: "fuzzy".to_string(),
                limit: 3,
            },
        )
        .unwrap();
        assert_eq!(truncated.total_matched, 7);
        assert_eq!(truncated.items.len(), 3);
        assert!(truncated.truncated);

        let exact_decorated = find_symbol(
            workspace.path().to_string_lossy().as_ref(),
            FindSymbolRequestPayload {
                symbol: "load_decorated".to_string(),
                path: Some("py/in_scope".to_string()),
                analyzer_language: "python".to_string(),
                public_language_filter: Some("python".to_string()),
                kind: "any".to_string(),
                match_mode: "exact".to_string(),
                limit: 50,
            },
        )
        .unwrap();
        assert_eq!(exact_decorated.total_matched, 1);
        assert_eq!(exact_decorated.items[0].symbol, "load_decorated");
    }

    #[test]
    fn supports_rust_exact_fuzzy_kind_path_truncation_and_unsupported_scopes() {
        let workspace = tempdir().unwrap();
        std::fs::create_dir_all(workspace.path().join("rust/in_scope")).unwrap();
        std::fs::create_dir_all(workspace.path().join("rust/out_scope")).unwrap();
        std::fs::create_dir_all(workspace.path().join("ts_only")).unwrap();
        std::fs::write(
            workspace.path().join("rust/in_scope/models.rs"),
            "pub struct UserId;\npub enum JobState { Ready }\npub trait Runner { fn run(&self); }\npub type LoadResult = String;\npub fn load() {}\npub fn load_data() {}\nimpl UserId {\n    #[allow(dead_code)]\n    pub fn load_cached(&self) {}\n}\n",
        )
        .unwrap();
        std::fs::write(
            workspace.path().join("rust/in_scope/service.rs"),
            "pub struct Loader;\npub struct LoaderFactory;\npub struct RunService;\nimpl RunService {\n    pub fn run() {}\n    #[cfg(test)]\n    pub fn load_attr(&self) {}\n}\npub fn run_task() {}\npub fn runner() {}\nfn outer() {\n    fn load_nested() {}\n}\n",
        )
        .unwrap();
        std::fs::write(
            workspace.path().join("rust/in_scope/unsupported.rs"),
            "macro_rules! nope { () => {} }\nmod nested {}\nunion Bits { value: u32 }\nconst LIMIT: u32 = 1;\nstatic NAME: &str = \"x\";\nextern \"C\" { fn ffi(); }\ntrait Shape {\n    fn area(&self);\n    const SIDES: usize;\n    type Output;\n}\n",
        )
        .unwrap();
        std::fs::write(
            workspace.path().join("rust/out_scope/other.rs"),
            "pub fn load_elsewhere() {}\n",
        )
        .unwrap();
        std::fs::write(
            workspace.path().join("ts_only/example.ts"),
            "export function AnalyzerRegistry() {}\n",
        )
        .unwrap();

        let exact_struct = find_symbol(
            workspace.path().to_string_lossy().as_ref(),
            FindSymbolRequestPayload {
                symbol: "Loader".to_string(),
                path: Some("rust/in_scope".to_string()),
                analyzer_language: "rust".to_string(),
                public_language_filter: Some("rust".to_string()),
                kind: "any".to_string(),
                match_mode: "exact".to_string(),
                limit: 50,
            },
        )
        .unwrap();
        assert_eq!(exact_struct.resolved_path.as_deref(), Some("rust/in_scope"));
        assert_eq!(exact_struct.total_matched, 1);
        assert_eq!(exact_struct.items[0].symbol, "Loader");
        assert_eq!(exact_struct.items[0].kind, "type");
        assert_eq!(exact_struct.items[0].language.as_deref(), Some("rust"));

        let exact_method = find_symbol(
            workspace.path().to_string_lossy().as_ref(),
            FindSymbolRequestPayload {
                symbol: "run".to_string(),
                path: Some("rust/in_scope".to_string()),
                analyzer_language: "rust".to_string(),
                public_language_filter: Some("rust".to_string()),
                kind: "any".to_string(),
                match_mode: "exact".to_string(),
                limit: 50,
            },
        )
        .unwrap();
        assert_eq!(exact_method.total_matched, 1);
        assert_eq!(exact_method.items[0].symbol, "run");
        assert_eq!(exact_method.items[0].kind, "method");

        let fuzzy = find_symbol(
            workspace.path().to_string_lossy().as_ref(),
            FindSymbolRequestPayload {
                symbol: "load".to_string(),
                path: Some("rust/in_scope".to_string()),
                analyzer_language: "rust".to_string(),
                public_language_filter: Some("rust".to_string()),
                kind: "any".to_string(),
                match_mode: "fuzzy".to_string(),
                limit: 50,
            },
        )
        .unwrap();
        let ordered = fuzzy
            .items
            .iter()
            .map(|item| {
                (
                    item.path.as_str(),
                    item.line,
                    item.symbol.as_str(),
                    item.kind.as_str(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            ordered,
            vec![
                ("rust/in_scope/models.rs", 4, "LoadResult", "type"),
                ("rust/in_scope/models.rs", 5, "load", "function"),
                ("rust/in_scope/models.rs", 6, "load_data", "function"),
                ("rust/in_scope/models.rs", 9, "load_cached", "method"),
                ("rust/in_scope/service.rs", 1, "Loader", "type"),
                ("rust/in_scope/service.rs", 2, "LoaderFactory", "type"),
                ("rust/in_scope/service.rs", 7, "load_attr", "method"),
            ]
        );

        let methods_only = find_symbol(
            workspace.path().to_string_lossy().as_ref(),
            FindSymbolRequestPayload {
                symbol: "load".to_string(),
                path: Some("rust/in_scope".to_string()),
                analyzer_language: "rust".to_string(),
                public_language_filter: Some("rust".to_string()),
                kind: "method".to_string(),
                match_mode: "fuzzy".to_string(),
                limit: 50,
            },
        )
        .unwrap();
        assert_eq!(methods_only.total_matched, 2);
        assert!(methods_only.items.iter().all(|item| item.kind == "method"));

        let kind_map = find_symbol(
            workspace.path().to_string_lossy().as_ref(),
            FindSymbolRequestPayload {
                symbol: "".to_string(),
                path: Some("rust/in_scope/models.rs".to_string()),
                analyzer_language: "rust".to_string(),
                public_language_filter: Some("rust".to_string()),
                kind: "any".to_string(),
                match_mode: "fuzzy".to_string(),
                limit: 50,
            },
        )
        .unwrap();
        let kinds = kind_map
            .items
            .iter()
            .map(|item| (item.symbol.as_str(), item.kind.as_str()))
            .collect::<Vec<_>>();
        assert!(kinds.contains(&("UserId", "type")));
        assert!(kinds.contains(&("JobState", "enum")));
        assert!(kinds.contains(&("Runner", "interface")));
        assert!(kinds.contains(&("LoadResult", "type")));
        assert!(kinds.contains(&("load", "function")));
        assert!(kinds.contains(&("load_cached", "method")));
        assert_eq!(
            kind_map
                .items
                .iter()
                .filter(|item| item.symbol == "load_attr")
                .count(),
            0
        );

        let truncated = find_symbol(
            workspace.path().to_string_lossy().as_ref(),
            FindSymbolRequestPayload {
                symbol: "load".to_string(),
                path: Some("rust/in_scope".to_string()),
                analyzer_language: "rust".to_string(),
                public_language_filter: Some("rust".to_string()),
                kind: "any".to_string(),
                match_mode: "fuzzy".to_string(),
                limit: 3,
            },
        )
        .unwrap();
        assert_eq!(truncated.total_matched, 7);
        assert_eq!(truncated.items.len(), 3);
        assert!(truncated.truncated);

        let attributed_impl = find_symbol(
            workspace.path().to_string_lossy().as_ref(),
            FindSymbolRequestPayload {
                symbol: "load_attr".to_string(),
                path: Some("rust/in_scope".to_string()),
                analyzer_language: "rust".to_string(),
                public_language_filter: Some("rust".to_string()),
                kind: "any".to_string(),
                match_mode: "exact".to_string(),
                limit: 50,
            },
        )
        .unwrap();
        assert_eq!(attributed_impl.total_matched, 1);
        assert_eq!(attributed_impl.items[0].kind, "method");

        let unsupported_only = find_symbol(
            workspace.path().to_string_lossy().as_ref(),
            FindSymbolRequestPayload {
                symbol: "Shape".to_string(),
                path: Some("rust/in_scope/unsupported.rs".to_string()),
                analyzer_language: "rust".to_string(),
                public_language_filter: Some("rust".to_string()),
                kind: "method".to_string(),
                match_mode: "fuzzy".to_string(),
                limit: 50,
            },
        )
        .unwrap();
        assert_eq!(unsupported_only.total_matched, 0);

        let non_rust_scope = find_symbol(
            workspace.path().to_string_lossy().as_ref(),
            FindSymbolRequestPayload {
                symbol: "AnalyzerRegistry".to_string(),
                path: Some("ts_only".to_string()),
                analyzer_language: "rust".to_string(),
                public_language_filter: Some("rust".to_string()),
                kind: "any".to_string(),
                match_mode: "exact".to_string(),
                limit: 50,
            },
        )
        .unwrap();
        assert_eq!(non_rust_scope.total_matched, 0);
        assert!(non_rust_scope.items.is_empty());
    }
}
