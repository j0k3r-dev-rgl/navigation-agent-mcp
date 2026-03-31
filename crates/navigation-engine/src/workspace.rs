use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::EngineError;

pub const DEFAULT_IGNORED_DIRECTORY_NAMES: [&str; 18] = [
    ".agent",
    ".agents",
    ".git",
    ".idea",
    "node_modules",
    ".react-router",
    ".vscode",
    ".claude",
    "build",
    "dist",
    ".next",
    "target",
    "coverage",
    ".turbo",
    ".cache",
    "tmp",
    "temp",
    "out",
];

#[derive(Debug, Clone)]
pub struct ResolvedScope {
    pub absolute_path: PathBuf,
    pub public_path: String,
    pub explicit: bool,
}

pub fn canonicalize_workspace_root(workspace_root: &str) -> Result<PathBuf, EngineError> {
    PathBuf::from(workspace_root)
        .canonicalize()
        .map_err(|_| EngineError::file_not_found(workspace_root))
}

pub fn resolve_scope(
    workspace_root: &Path,
    requested_path: Option<&str>,
) -> Result<ResolvedScope, EngineError> {
    let requested_path = requested_path.unwrap_or(".");
    let candidate = PathBuf::from(requested_path);
    let resolved = if candidate.is_absolute() {
        normalize_path(candidate)
    } else {
        normalize_path(workspace_root.join(candidate))
    };

    if resolved != workspace_root && !resolved.starts_with(workspace_root) {
        return Err(EngineError::path_outside_workspace(requested_path));
    }

    let canonical = resolved
        .canonicalize()
        .map_err(|_| EngineError::file_not_found(requested_path))?;

    Ok(ResolvedScope {
        public_path: public_path(workspace_root, &canonical),
        absolute_path: canonical,
        explicit: requested_path != ".",
    })
}

pub fn contains_hard_ignored_segment(workspace_root: &Path, root_path: &Path) -> bool {
    if root_path == workspace_root {
        return false;
    }

    root_path
        .strip_prefix(workspace_root)
        .ok()
        .map(|relative| {
            relative.components().any(|component| {
                DEFAULT_IGNORED_DIRECTORY_NAMES
                    .contains(&component.as_os_str().to_string_lossy().as_ref())
            })
        })
        .unwrap_or(false)
}

pub fn ignored_directories() -> Vec<String> {
    DEFAULT_IGNORED_DIRECTORY_NAMES
        .iter()
        .map(|value| value.to_string())
        .collect()
}

pub fn should_ignore_name(name: &str, include_hidden: bool) -> bool {
    if DEFAULT_IGNORED_DIRECTORY_NAMES.contains(&name) {
        return true;
    }

    !include_hidden && name.starts_with('.')
}

pub fn public_path(workspace_root: &Path, path: &Path) -> String {
    if path == workspace_root {
        return ".".to_string();
    }

    path.strip_prefix(workspace_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

pub fn normalize_path(path: PathBuf) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::CurDir => {}
            other => normalized.push(other.as_os_str()),
        }
    }

    normalized
}

pub fn collect_supported_source_files(
    workspace_root: &Path,
    scope: &ResolvedScope,
    supported_extensions: &BTreeSet<String>,
    include_hidden: bool,
) -> Result<Vec<PathBuf>, EngineError> {
    if contains_hard_ignored_segment(workspace_root, &scope.absolute_path) {
        return Ok(Vec::new());
    }

    if scope.absolute_path.is_file() {
        if is_supported_file(&scope.absolute_path, supported_extensions) {
            return Ok(vec![scope.absolute_path.clone()]);
        }

        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    walk_supported_files(
        &scope.absolute_path,
        supported_extensions,
        include_hidden,
        &mut files,
    )?;
    files.sort_by(|left, right| left.to_string_lossy().cmp(&right.to_string_lossy()));
    Ok(files)
}

fn walk_supported_files(
    current_path: &Path,
    supported_extensions: &BTreeSet<String>,
    include_hidden: bool,
    files: &mut Vec<PathBuf>,
) -> Result<(), EngineError> {
    let read_dir = match fs::read_dir(current_path) {
        Ok(rd) => rd,
        Err(err) => {
            // Log permission errors to stderr but continue gracefully
            if err.kind() == std::io::ErrorKind::PermissionDenied {
                eprintln!(
                    "Warning: Permission denied reading directory: {}",
                    current_path.display()
                );
            }
            // Silently skip directories we cannot read
            return Ok(());
        }
    };
    let mut entries = read_dir.filter_map(Result::ok).collect::<Vec<_>>();

    entries.sort_by(|left, right| {
        let left_is_dir = left
            .file_type()
            .map(|value| value.is_dir())
            .unwrap_or(false);
        let right_is_dir = right
            .file_type()
            .map(|value| value.is_dir())
            .unwrap_or(false);
        (
            !left_is_dir,
            left.file_name().to_string_lossy().to_lowercase(),
        )
            .cmp(&(
                !right_is_dir,
                right.file_name().to_string_lossy().to_lowercase(),
            ))
    });

    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if should_ignore_name(&name, include_hidden) {
            continue;
        }

        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        let entry_path = entry.path();
        if file_type.is_dir() {
            if !file_type.is_symlink() {
                walk_supported_files(&entry_path, supported_extensions, include_hidden, files)?;
            }
            continue;
        }

        if is_supported_file(&entry_path, supported_extensions) {
            files.push(entry_path);
        }
    }

    Ok(())
}

fn is_supported_file(path: &Path, supported_extensions: &BTreeSet<String>) -> bool {
    let Some(extension) = path.extension() else {
        return false;
    };

    supported_extensions.contains(&format!(".{}", extension.to_string_lossy().to_lowercase()))
}
