use crate::error::EngineError;
use crate::protocol::{
    EngineRequest, EngineResponse, InspectTreeItem, InspectTreeItemStats, InspectTreeRequestPayload,
    InspectTreeResult,
};
use crate::workspace::{
    canonicalize_workspace_root, contains_hard_ignored_segment, ignored_directories, public_path,
    resolve_scope, should_ignore_name,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

pub const CAPABILITY: &str = "workspace.inspect_tree";
pub const MAX_TREE_ITEMS: usize = 2000;

pub fn handle(request: EngineRequest) -> EngineResponse {
    let parsed_payload = serde_json::from_value::<InspectTreeRequestPayload>(request.payload.clone());

    match parsed_payload {
        Ok(payload) => match inspect_tree(&request.workspace_root, payload) {
            Ok(result) => EngineResponse::success(request.id, &result),
            Err(error) => EngineResponse::error(request.id, error),
        },
        Err(error) => EngineResponse::error(
            request.id,
            EngineError::invalid_request(error.to_string()),
        ),
    }
}

fn inspect_tree(workspace_root: &str, payload: InspectTreeRequestPayload) -> Result<InspectTreeResult, EngineError> {
    let workspace_root = canonicalize_workspace_root(workspace_root)?;
    let scope = resolve_scope(&workspace_root, payload.path.as_deref())?;
    let root_path = scope.absolute_path;
    let root_relative = scope.public_path;

    if contains_hard_ignored_segment(&workspace_root, &root_path) {
        return Ok(InspectTreeResult {
            root: root_relative,
            items: Vec::new(),
            truncated: false,
            max_items: MAX_TREE_ITEMS,
            ignored_directories: ignored_directories(),
        });
    }

    let normalized_extensions = normalize_extensions(payload.extensions);

    if root_path.is_file() {
        let extension = root_path.extension().map(|value| format!(".{}", value.to_string_lossy().to_lowercase()));
        let mut items = Vec::new();
        if matches_filters(
            false,
            root_path.file_name().map(|value| value.to_string_lossy().to_string()).unwrap_or_default().as_str(),
            extension.as_deref(),
            &normalized_extensions,
            payload.file_pattern.as_deref(),
        ) {
            items.push(build_item(&workspace_root, &root_path, 1, payload.include_stats)?);
        }

        return Ok(InspectTreeResult {
            root: root_relative,
            items,
            truncated: false,
            max_items: MAX_TREE_ITEMS,
            ignored_directories: ignored_directories(),
        });
    }

    let mut items = Vec::new();
    let mut truncated = false;
    walk(
        &workspace_root,
        &root_path,
        &root_path,
        0,
        payload.max_depth,
        payload.include_hidden,
        payload.include_stats,
        &normalized_extensions,
        payload.file_pattern.as_deref(),
        &mut items,
        &mut truncated,
    )?;

    Ok(InspectTreeResult {
        root: root_relative,
        items,
        truncated,
        max_items: MAX_TREE_ITEMS,
        ignored_directories: ignored_directories(),
    })
}

#[allow(clippy::too_many_arguments)]
fn walk(
    workspace_root: &Path,
    root_path: &Path,
    current_path: &Path,
    current_depth: u32,
    max_depth: u32,
    include_hidden: bool,
    include_stats: bool,
    normalized_extensions: &BTreeSet<String>,
    file_pattern: Option<&str>,
    items: &mut Vec<InspectTreeItem>,
    truncated: &mut bool,
) -> Result<(), EngineError> {
    if *truncated || current_depth >= max_depth {
        return Ok(());
    }

    let mut entries = fs::read_dir(current_path)
        .map_err(|error| EngineError::backend_execution_failed(error.to_string()))?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    entries.sort_by(|left, right| {
        let left_is_dir = left.file_type().map(|value| value.is_dir()).unwrap_or(false);
        let right_is_dir = right.file_type().map(|value| value.is_dir()).unwrap_or(false);
        (!left_is_dir, left.file_name().to_string_lossy().to_lowercase())
            .cmp(&(!right_is_dir, right.file_name().to_string_lossy().to_lowercase()))
    });

    for entry in entries {
        if *truncated {
            return Ok(());
        }

        let name = entry.file_name().to_string_lossy().to_string();
        if should_ignore_name(&name, include_hidden) {
            continue;
        }

        let entry_path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| EngineError::backend_execution_failed(error.to_string()))?;
        let is_directory = file_type.is_dir();
        let item_depth = entry_path
            .strip_prefix(root_path)
            .unwrap_or(&entry_path)
            .components()
            .count() as u32;
        let extension = entry_path.extension().map(|value| format!(".{}", value.to_string_lossy().to_lowercase()));

        if matches_filters(
            is_directory,
            &name,
            extension.as_deref(),
            normalized_extensions,
            file_pattern,
        ) {
            items.push(build_item(workspace_root, &entry_path, item_depth, include_stats)?);
            if items.len() >= MAX_TREE_ITEMS {
              *truncated = true;
              return Ok(());
            }
        }

        if is_directory && item_depth < max_depth && !file_type.is_symlink() {
            walk(
                workspace_root,
                root_path,
                &entry_path,
                current_depth + 1,
                max_depth,
                include_hidden,
                include_stats,
                normalized_extensions,
                file_pattern,
                items,
                truncated,
            )?;
        }
    }

    Ok(())
}

fn build_item(
    workspace_root: &Path,
    entry_path: &Path,
    depth: u32,
    include_stats: bool,
) -> Result<InspectTreeItem, EngineError> {
    let metadata = fs::symlink_metadata(entry_path)
        .map_err(|error| EngineError::backend_execution_failed(error.to_string()))?;
    let is_directory = metadata.is_dir();
    let extension = if is_directory {
        None
    } else {
        entry_path
            .extension()
            .map(|value| format!(".{}", value.to_string_lossy().to_lowercase()))
    };

    Ok(InspectTreeItem {
        path: public_path(workspace_root, entry_path),
        name: entry_path
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default(),
        item_type: if is_directory { "directory" } else { "file" }.to_string(),
        depth,
        extension,
        stats: if include_stats {
            Some(InspectTreeItemStats {
                size_bytes: metadata.len(),
                modified_at: metadata
                    .modified()
                    .ok()
                    .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
                    .map(|value| chrono_like_iso(value.as_secs()))
                    .unwrap_or_else(|| "1970-01-01T00:00:00+00:00".to_string()),
                is_symlink: metadata.file_type().is_symlink(),
            })
        } else {
            None
        },
    })
}

fn chrono_like_iso(seconds: u64) -> String {
    use std::time::{Duration, UNIX_EPOCH};

    let datetime = UNIX_EPOCH + Duration::from_secs(seconds);
    let system_time: chrono::DateTime<chrono::Utc> = datetime.into();
    system_time.to_rfc3339()
}

fn matches_filters(
    is_directory: bool,
    name: &str,
    extension: Option<&str>,
    normalized_extensions: &BTreeSet<String>,
    file_pattern: Option<&str>,
) -> bool {
    if is_directory {
        return true;
    }
    if !normalized_extensions.is_empty() && !extension.is_some_and(|value| normalized_extensions.contains(value)) {
        return false;
    }
    if let Some(pattern) = file_pattern {
        return glob_match(pattern, name);
    }
    true
}

fn glob_match(pattern: &str, text: &str) -> bool {
    glob::Pattern::new(pattern)
        .map(|value| value.matches(text))
        .unwrap_or(false)
}

fn normalize_extensions(extensions: Vec<String>) -> BTreeSet<String> {
    extensions.into_iter().map(|value| value.to_lowercase()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn hidden_files_respect_hard_ignores() {
        let workspace = tempdir().unwrap();
        std::fs::create_dir_all(workspace.path().join("src")).unwrap();
        std::fs::create_dir_all(workspace.path().join(".hidden")).unwrap();
        std::fs::create_dir_all(workspace.path().join(".git")).unwrap();
        std::fs::create_dir_all(workspace.path().join("node_modules")).unwrap();
        std::fs::write(workspace.path().join("src/main.py"), "ok\n").unwrap();
        std::fs::write(workspace.path().join(".hidden/note.txt"), "secret\n").unwrap();
        std::fs::write(workspace.path().join(".git/config"), "[core]\n").unwrap();

        let result = inspect_tree(
            workspace.path().to_string_lossy().as_ref(),
            InspectTreeRequestPayload {
                path: None,
                max_depth: 2,
                extensions: vec![],
                file_pattern: None,
                include_stats: false,
                include_hidden: true,
            },
        )
        .unwrap();

        let paths = result.items.iter().map(|item| item.path.as_str()).collect::<Vec<_>>();
        assert!(paths.contains(&".hidden"));
        assert!(paths.contains(&".hidden/note.txt"));
        assert!(!paths.iter().any(|path| *path == ".git" || path.starts_with(".git/")));
        assert!(!paths.iter().any(|path| *path == "node_modules" || path.starts_with("node_modules/")));
    }
}
