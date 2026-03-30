from __future__ import annotations

import json
import shutil
import subprocess
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from navigation_mcp.contracts.common import ErrorCode
from navigation_mcp.adapters.internal_tools.find_symbol_adapter import BackendToolError


@dataclass(slots=True)
class InternalSearchTextResult:
    files: list[dict[str, Any]]
    file_count: int
    match_count: int
    raw_events: int


class InternalSearchTextAdapter:
    def search_text(
        self,
        *,
        workspace_root: Path,
        query: str,
        path_filter: Path | None,
        include: str | None,
        regex: bool,
        context: int,
        language_globs: list[str],
    ) -> InternalSearchTextResult:
        if shutil.which("rg") is None:
            raise BackendToolError(
                code=ErrorCode.BACKEND_DEPENDENCY_NOT_FOUND,
                message="ripgrep (rg) is required for text search but is not installed.",
                retryable=False,
                suggestion="Install ripgrep and retry the request.",
                details={"dependency": "rg"},
            )

        command = [
            "rg",
            "--json",
            "--line-number",
            "--color",
            "never",
            "--context",
            str(context),
        ]

        if not regex:
            command.append("--fixed-strings")

        glob_filters = [*language_globs]
        if include:
            glob_filters.append(include)
        for glob_filter in glob_filters:
            command.extend(["--glob", glob_filter])

        search_root = workspace_root
        if path_filter is not None:
            search_root = workspace_root / path_filter
            if not search_root.exists():
                return InternalSearchTextResult(
                    files=[],
                    file_count=0,
                    match_count=0,
                    raw_events=0,
                )

        command.extend([query, str(search_root)])

        process = subprocess.run(
            command,
            capture_output=True,
            text=True,
            cwd=str(workspace_root),
            check=False,
        )

        stdout_lines = [line for line in process.stdout.splitlines() if line.strip()]
        if process.returncode not in {0, 1}:
            details: dict[str, Any] = {
                "returncode": process.returncode,
            }
            stderr = process.stderr.strip()
            if stderr:
                details["stderr"] = stderr
            raise BackendToolError(
                code=ErrorCode.BACKEND_EXECUTION_FAILED,
                message="Internal text search adapter failed to execute.",
                retryable=True,
                suggestion="Verify the search query and ripgrep availability, then retry.",
                details=details,
            )

        files_by_path: dict[str, dict[str, Any]] = {}
        pending_before: dict[str, list[dict[str, Any]]] = defaultdict(list)
        match_count = 0

        for line in stdout_lines:
            try:
                event = json.loads(line)
            except json.JSONDecodeError as exc:
                raise BackendToolError(
                    code=ErrorCode.BACKEND_INVALID_RESPONSE,
                    message="Internal text search adapter returned invalid JSON.",
                    retryable=True,
                    suggestion="Inspect the ripgrep JSON stream and adapter normalization.",
                    details={"line": line},
                ) from exc

            event_type = event.get("type")
            data = event.get("data", {})
            path_text = self._extract_text(data.get("path", {}))
            if not path_text:
                continue

            relative_path = self._normalize_relative_path(workspace_root, path_text)
            file_entry = files_by_path.setdefault(
                relative_path,
                {"path": relative_path, "matches": []},
            )

            if event_type == "context":
                line_number = data.get("line_number")
                context_line = {
                    "line": int(line_number or 1),
                    "text": self._extract_text(data.get("lines", {})),
                }
                if data.get("submatches"):
                    pending_before[relative_path].append(context_line)
                    continue

                target_match = self._find_target_match(
                    matches=file_entry["matches"],
                    context_line=context_line,
                    context=context,
                )
                if target_match is None:
                    pending_before[relative_path].append(context_line)
                    continue
                target_match["after"].append(context_line)
                continue

            if event_type != "match":
                continue

            line_number = int(data.get("line_number") or 1)
            match = {
                "line": line_number,
                "text": self._extract_text(data.get("lines", {})),
                "submatches": [
                    {
                        "start": int(item.get("start", 0)),
                        "end": int(item.get("end", 0)),
                        "text": self._extract_text(item.get("match", {})),
                    }
                    for item in data.get("submatches", [])
                ],
                "before": pending_before.pop(relative_path, []),
                "after": [],
            }
            file_entry["matches"].append(match)
            match_count += 1

        files = []
        for file_entry in sorted(files_by_path.values(), key=lambda item: item["path"]):
            if not file_entry["matches"]:
                continue
            for match in file_entry["matches"]:
                match["before"] = sorted(match["before"], key=lambda item: item["line"])
                match["after"] = sorted(match["after"], key=lambda item: item["line"])
            file_entry["matches"] = sorted(
                file_entry["matches"], key=lambda item: item["line"]
            )
            file_entry["matchCount"] = len(file_entry["matches"])
            files.append(file_entry)

        return InternalSearchTextResult(
            files=files,
            file_count=len(files),
            match_count=match_count,
            raw_events=len(stdout_lines),
        )

    def _extract_text(self, value: dict[str, Any]) -> str:
        text = value.get("text")
        if isinstance(text, str):
            return text.rstrip("\n\r")
        return ""

    def _normalize_relative_path(self, workspace_root: Path, path_value: str) -> str:
        candidate = Path(path_value)
        if candidate.is_absolute():
            try:
                return candidate.relative_to(workspace_root).as_posix()
            except ValueError:
                return candidate.as_posix()
        return candidate.as_posix()

    def _find_target_match(
        self,
        *,
        matches: list[dict[str, Any]],
        context_line: dict[str, Any],
        context: int,
    ) -> dict[str, Any] | None:
        line = int(context_line["line"])
        for match in reversed(matches):
            distance = line - int(match["line"])
            if 1 <= distance <= context:
                return match
        return None
