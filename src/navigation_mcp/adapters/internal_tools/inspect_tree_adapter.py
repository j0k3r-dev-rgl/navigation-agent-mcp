from __future__ import annotations

from dataclasses import dataclass
from datetime import UTC, datetime
import fnmatch
import os
from pathlib import Path


DEFAULT_IGNORED_DIRECTORY_NAMES = frozenset(
    {
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
    }
)

MAX_TREE_ITEMS = 2000


@dataclass(slots=True)
class InternalInspectTreeResult:
    root: str
    items: list[dict[str, object]]
    truncated: bool
    max_items: int
    ignored_directories: list[str]


class InternalInspectTreeAdapter:
    def inspect_tree(
        self,
        *,
        workspace_root: Path,
        root_path: Path,
        max_depth: int,
        extensions: list[str],
        file_pattern: str | None,
        include_stats: bool,
        include_hidden: bool,
    ) -> InternalInspectTreeResult:
        relative_root = (
            root_path.relative_to(workspace_root).as_posix()
            if root_path != workspace_root
            else "."
        )
        if self._path_contains_hard_ignored_segment(workspace_root, root_path):
            return InternalInspectTreeResult(
                root=relative_root,
                items=[],
                truncated=False,
                max_items=MAX_TREE_ITEMS,
                ignored_directories=sorted(DEFAULT_IGNORED_DIRECTORY_NAMES),
            )

        items: list[dict[str, object]] = []
        truncated = False

        normalized_extensions = {extension.lower() for extension in extensions}

        if root_path.is_file():
            suffix = root_path.suffix.lower()
            if self._matches_filters(
                is_directory=False,
                name=root_path.name,
                extension=suffix,
                normalized_extensions=normalized_extensions,
                file_pattern=file_pattern,
            ):
                items.append(
                    self._build_path_item(
                        workspace_root=workspace_root,
                        entry_path=root_path,
                        name=root_path.name,
                        is_directory=False,
                        depth=1,
                        extension=suffix or None,
                        include_stats=include_stats,
                    )
                )
            return InternalInspectTreeResult(
                root=relative_root,
                items=items,
                truncated=False,
                max_items=MAX_TREE_ITEMS,
                ignored_directories=sorted(DEFAULT_IGNORED_DIRECTORY_NAMES),
            )

        def walk(current_path: Path, current_depth: int) -> None:
            nonlocal truncated
            if truncated or current_depth >= max_depth:
                return

            try:
                entries = sorted(
                    os.scandir(current_path),
                    key=lambda entry: (
                        not self._is_directory(entry),
                        entry.name.lower(),
                    ),
                )
            except OSError:
                return

            for entry in entries:
                if truncated:
                    return
                if self._should_ignore(entry.name, include_hidden=include_hidden):
                    continue

                entry_path = Path(entry.path)
                relative_path = entry_path.relative_to(workspace_root).as_posix()
                is_directory = self._is_directory(entry)
                item_depth = len(entry_path.relative_to(root_path).parts)

                if self._matches_filters(
                    is_directory=is_directory,
                    name=entry.name,
                    extension=entry_path.suffix.lower(),
                    normalized_extensions=normalized_extensions,
                    file_pattern=file_pattern,
                ):
                    items.append(
                        self._build_item(
                            path=relative_path,
                            name=entry.name,
                            is_directory=is_directory,
                            depth=item_depth,
                            extension=entry_path.suffix.lower() or None,
                            include_stats=include_stats,
                            dir_entry=entry,
                        )
                    )
                    if len(items) >= MAX_TREE_ITEMS:
                        truncated = True
                        return

                if is_directory and item_depth < max_depth and not entry.is_symlink():
                    walk(entry_path, current_depth + 1)

        walk(root_path, current_depth=0)

        return InternalInspectTreeResult(
            root=relative_root,
            items=items,
            truncated=truncated,
            max_items=MAX_TREE_ITEMS,
            ignored_directories=sorted(DEFAULT_IGNORED_DIRECTORY_NAMES),
        )

    def _path_contains_hard_ignored_segment(
        self, workspace_root: Path, root_path: Path
    ) -> bool:
        if root_path == workspace_root:
            return False
        return any(
            part in DEFAULT_IGNORED_DIRECTORY_NAMES
            for part in root_path.relative_to(workspace_root).parts
        )

    def _should_ignore(self, name: str, *, include_hidden: bool) -> bool:
        if name in DEFAULT_IGNORED_DIRECTORY_NAMES:
            return True
        if not include_hidden and name.startswith("."):
            return True
        return False

    def _matches_filters(
        self,
        *,
        is_directory: bool,
        name: str,
        extension: str,
        normalized_extensions: set[str],
        file_pattern: str | None,
    ) -> bool:
        if is_directory:
            return True
        if normalized_extensions and extension not in normalized_extensions:
            return False
        if file_pattern and not fnmatch.fnmatch(name, file_pattern):
            return False
        return True

    def _build_item(
        self,
        *,
        path: str,
        name: str,
        is_directory: bool,
        depth: int,
        extension: str | None,
        include_stats: bool,
        dir_entry: os.DirEntry[str],
    ) -> dict[str, object]:
        item: dict[str, object] = {
            "path": path,
            "name": name,
            "type": "directory" if is_directory else "file",
            "depth": depth,
        }
        if not is_directory and extension:
            item["extension"] = extension
        if include_stats:
            item["stats"] = self._build_stats(dir_entry)
        return item

    def _build_path_item(
        self,
        *,
        workspace_root: Path,
        entry_path: Path,
        name: str,
        is_directory: bool,
        depth: int,
        extension: str | None,
        include_stats: bool,
    ) -> dict[str, object]:
        item: dict[str, object] = {
            "path": entry_path.relative_to(workspace_root).as_posix(),
            "name": name,
            "type": "directory" if is_directory else "file",
            "depth": depth,
        }
        if not is_directory and extension:
            item["extension"] = extension
        if include_stats:
            stat_result = entry_path.lstat()
            item["stats"] = {
                "sizeBytes": int(stat_result.st_size),
                "modifiedAt": datetime.fromtimestamp(
                    stat_result.st_mtime, tz=UTC
                ).isoformat(),
                "isSymlink": entry_path.is_symlink(),
            }
        return item

    def _build_stats(self, dir_entry: os.DirEntry[str]) -> dict[str, object]:
        stat_result = dir_entry.stat(follow_symlinks=False)
        modified_at = datetime.fromtimestamp(stat_result.st_mtime, tz=UTC).isoformat()
        return {
            "sizeBytes": int(stat_result.st_size),
            "modifiedAt": modified_at,
            "isSymlink": dir_entry.is_symlink(),
        }

    def _is_directory(self, dir_entry: os.DirEntry[str]) -> bool:
        try:
            return dir_entry.is_dir(follow_symlinks=True)
        except OSError:
            return False
