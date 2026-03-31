use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

use crate::error::EngineError;
use crate::protocol::{
    EngineRequest, EngineResponse, SearchTextContextLine, SearchTextFileMatch, SearchTextMatch,
    SearchTextRequestPayload, SearchTextResult, SearchTextSubmatch,
};
use crate::workspace::{
    canonicalize_workspace_root, contains_hard_ignored_segment, public_path, resolve_scope,
    DEFAULT_IGNORED_DIRECTORY_NAMES,
};

pub const CAPABILITY: &str = "workspace.search_text";

pub fn handle(request: EngineRequest) -> EngineResponse {
    let parsed_payload =
        serde_json::from_value::<SearchTextRequestPayload>(request.payload.clone());

    match parsed_payload {
        Ok(payload) => match search_text(&request.workspace_root, payload) {
            Ok(result) => EngineResponse::success(request.id, &result),
            Err(error) => EngineResponse::error(request.id, error),
        },
        Err(error) => {
            EngineResponse::error(request.id, EngineError::invalid_request(error.to_string()))
        }
    }
}

pub fn search_text(
    workspace_root: &str,
    payload: SearchTextRequestPayload,
) -> Result<SearchTextResult, EngineError> {
    let workspace_root = canonicalize_workspace_root(workspace_root)?;
    let scope = resolve_scope(&workspace_root, payload.path.as_deref())?;

    if contains_hard_ignored_segment(&workspace_root, &scope.absolute_path) {
        return Ok(SearchTextResult {
            resolved_path: scope.explicit.then_some(scope.public_path),
            items: Vec::new(),
            total_file_count: 0,
            total_match_count: 0,
            truncated: false,
        });
    }

    let mut command = Command::new("rg");
    command
        .arg("--json")
        .arg("--line-number")
        .arg("--color")
        .arg("never")
        .arg("--context")
        .arg(payload.context.to_string());

    if !payload.regex {
        command.arg("--fixed-strings");
    }

    for glob in build_globs(
        payload.public_language_filter.as_deref(),
        payload.include.as_deref(),
    ) {
        command.arg("--glob").arg(glob);
    }
    for glob in ignored_globs() {
        command.arg("--glob").arg(glob);
    }

    command
        .arg(&payload.query)
        .arg(&scope.absolute_path)
        .current_dir(&workspace_root);

    let output = command.output().map_err(map_spawn_error)?;
    if !matches!(output.status.code(), Some(0 | 1)) {
        let mut details = serde_json::Map::new();
        if let Some(code) = output.status.code() {
            details.insert("returncode".to_string(), Value::from(code));
        }
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if !stderr.is_empty() {
            details.insert("stderr".to_string(), Value::from(stderr));
        }

        return Err(EngineError {
            code: "BACKEND_EXECUTION_FAILED".to_string(),
            message: "Internal text search adapter failed to execute.".to_string(),
            retryable: true,
            suggestion: Some(
                "Verify the search query and ripgrep availability, then retry.".to_string(),
            ),
            details: Value::Object(details),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut files_by_path: BTreeMap<String, SearchFileAccumulator> = BTreeMap::new();
    let mut pending_before: BTreeMap<String, Vec<SearchTextContextLine>> = BTreeMap::new();
    let mut total_match_count = 0usize;

    for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
        let event: Value = serde_json::from_str(line).map_err(|_| EngineError {
            code: "BACKEND_INVALID_RESPONSE".to_string(),
            message: "Internal text search adapter returned invalid JSON.".to_string(),
            retryable: true,
            suggestion: Some(
                "Inspect the ripgrep JSON stream and adapter normalization.".to_string(),
            ),
            details: serde_json::json!({ "line": line }),
        })?;

        let event_type = event
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let data = event.get("data").unwrap_or(&Value::Null);
        let Some(path_text) = extract_text(data.get("path")) else {
            continue;
        };

        let relative_path = normalize_relative_path(&workspace_root, &path_text);
        let entry = files_by_path
            .entry(relative_path.clone())
            .or_insert_with(|| SearchFileAccumulator::new(relative_path.clone()));

        if event_type == "context" {
            let context_line = SearchTextContextLine {
                line: extract_u32(data.get("line_number")).unwrap_or(1),
                text: extract_text(data.get("lines")).unwrap_or_default(),
            };

            if has_submatches(data.get("submatches")) {
                pending_before
                    .entry(relative_path)
                    .or_default()
                    .push(context_line);
                continue;
            }

            if let Some(target_match) =
                find_target_match(&mut entry.matches, &context_line, payload.context)
            {
                target_match.after.push(context_line);
            } else {
                pending_before
                    .entry(relative_path)
                    .or_default()
                    .push(context_line);
            }
            continue;
        }

        if event_type != "match" {
            continue;
        }

        let match_record = SearchTextMatch {
            line: extract_u32(data.get("line_number")).unwrap_or(1),
            text: extract_text(data.get("lines")).unwrap_or_default(),
            submatches: data
                .get("submatches")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(|item| SearchTextSubmatch {
                    start: item.get("start").and_then(Value::as_u64).unwrap_or(0) as usize,
                    end: item.get("end").and_then(Value::as_u64).unwrap_or(0) as usize,
                    text: extract_text(item.get("match")).unwrap_or_default(),
                })
                .collect(),
            before: pending_before.remove(&relative_path).unwrap_or_default(),
            after: Vec::new(),
        };

        entry.matches.push(match_record);
        total_match_count += 1;
    }

    let mut items = files_by_path
        .into_values()
        .filter_map(|mut entry| {
            if entry.matches.is_empty() {
                return None;
            }

            for match_record in &mut entry.matches {
                match_record.before.sort_by_key(|item| item.line);
                match_record.after.sort_by_key(|item| item.line);
            }
            entry.matches.sort_by_key(|item| item.line);

            Some(SearchTextFileMatch {
                language: infer_language_from_path(&entry.path),
                match_count: entry.matches.len(),
                path: entry.path,
                matches: entry.matches,
            })
        })
        .collect::<Vec<_>>();

    let total_file_count = items.len();
    let truncated = total_file_count > payload.limit;
    if truncated {
        items.truncate(payload.limit);
    }

    Ok(SearchTextResult {
        resolved_path: scope.explicit.then_some(scope.public_path),
        items,
        total_file_count,
        total_match_count,
        truncated,
    })
}

#[derive(Debug, Clone)]
struct SearchFileAccumulator {
    path: String,
    matches: Vec<SearchTextMatch>,
}

impl SearchFileAccumulator {
    fn new(path: String) -> Self {
        Self {
            path,
            matches: Vec::new(),
        }
    }
}

fn build_globs(language: Option<&str>, include: Option<&str>) -> Vec<String> {
    let mut globs = match language {
        Some("typescript") => vec!["*.ts".to_string(), "*.tsx".to_string()],
        Some("javascript") => vec!["*.js".to_string(), "*.jsx".to_string()],
        Some("java") => vec!["*.java".to_string()],
        Some("python") => vec!["*.py".to_string()],
        Some("rust") => vec!["*.rs".to_string()],
        _ => Vec::new(),
    };

    if let Some(include) = include.filter(|value| !value.trim().is_empty()) {
        globs.push(include.to_string());
    }

    globs
}

fn ignored_globs() -> Vec<String> {
    DEFAULT_IGNORED_DIRECTORY_NAMES
        .iter()
        .flat_map(|name| [format!("!{name}/**"), format!("!**/{name}/**")])
        .collect()
}

fn map_spawn_error(error: std::io::Error) -> EngineError {
    if error.kind() == std::io::ErrorKind::NotFound {
        return EngineError {
            code: "BACKEND_DEPENDENCY_NOT_FOUND".to_string(),
            message: "ripgrep (rg) is required for text search but is not installed.".to_string(),
            retryable: false,
            suggestion: Some("Install ripgrep and retry the request.".to_string()),
            details: serde_json::json!({ "dependency": "rg" }),
        };
    }

    EngineError::backend_execution_failed(error.to_string())
}

fn extract_text(value: Option<&Value>) -> Option<String> {
    value
        .and_then(|value| value.get("text"))
        .and_then(Value::as_str)
        .map(|value| value.trim_end_matches(['\n', '\r']).to_string())
}

fn extract_u32(value: Option<&Value>) -> Option<u32> {
    value.and_then(Value::as_u64).map(|value| value as u32)
}

fn has_submatches(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_array)
        .map(|items| !items.is_empty())
        .unwrap_or(false)
}

fn normalize_relative_path(workspace_root: &Path, path_value: &str) -> String {
    let candidate = Path::new(path_value);
    if candidate.is_absolute() {
        if let Ok(relative) = candidate.strip_prefix(workspace_root) {
            return public_path(workspace_root, relative);
        }
    }

    candidate.to_string_lossy().replace('\\', "/")
}

fn infer_language_from_path(path: &str) -> Option<String> {
    let extension = Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())?;

    match extension.as_str() {
        "ts" | "tsx" => Some("typescript".to_string()),
        "js" | "jsx" => Some("javascript".to_string()),
        "java" => Some("java".to_string()),
        "py" => Some("python".to_string()),
        "rs" => Some("rust".to_string()),
        _ => None,
    }
}

fn find_target_match<'a>(
    matches: &'a mut [SearchTextMatch],
    context_line: &SearchTextContextLine,
    context: usize,
) -> Option<&'a mut SearchTextMatch> {
    let line = context_line.line as i64;
    matches.iter_mut().rev().find(|match_record| {
        let distance = line - match_record.line as i64;
        (1..=context as i64).contains(&distance)
    })
}
