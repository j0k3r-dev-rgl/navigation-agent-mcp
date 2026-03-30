from __future__ import annotations

import json
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from navigation_mcp.adapters.internal_tools.find_symbol_adapter import BackendToolError
from navigation_mcp.contracts.common import ErrorCode

DEFAULT_TRACE_SYMBOL_SCRIPT = Path("/home/j0k3r/.config/opencode/tools/trace_symbol.py")


@dataclass(slots=True)
class InternalTraceSymbolResult:
    files: list[str]
    count: int
    options: dict[str, Any]


class InternalTraceSymbolAdapter:
    def __init__(self, script_path: Path | None = None) -> None:
        self.script_path = (
            (script_path or DEFAULT_TRACE_SYMBOL_SCRIPT).expanduser().resolve()
        )

    def trace_symbol(
        self,
        *,
        workspace_root: Path,
        path: Path,
        symbol: str,
        options: dict[str, Any],
    ) -> InternalTraceSymbolResult:
        if not self.script_path.exists():
            raise BackendToolError(
                code=ErrorCode.BACKEND_SCRIPT_NOT_FOUND,
                message="Internal symbol trace analyzer is not available.",
                retryable=False,
                suggestion="Install or configure the internal symbol trace analyzer and retry.",
            )

        absolute_path = (workspace_root / path).resolve()
        process = subprocess.run(
            [
                sys.executable,
                str(self.script_path),
                str(workspace_root),
                str(absolute_path),
                symbol,
                json.dumps(options),
            ],
            capture_output=True,
            text=True,
            cwd=str(workspace_root),
            check=False,
        )

        payload_text = process.stdout.strip() or process.stderr.strip()
        if not payload_text:
            raise BackendToolError(
                code=ErrorCode.BACKEND_INVALID_RESPONSE,
                message="Internal adapter returned no output.",
                retryable=True,
                suggestion="Retry the request or inspect the internal analyzer installation.",
                details={"returncode": process.returncode},
            )

        try:
            payload = json.loads(payload_text)
        except json.JSONDecodeError as exc:
            raise BackendToolError(
                code=ErrorCode.BACKEND_INVALID_RESPONSE,
                message="Internal adapter returned invalid JSON.",
                retryable=True,
                suggestion="Inspect the internal adapter output and ensure it emits valid JSON.",
                details={"returncode": process.returncode},
            ) from exc

        if process.returncode != 0 or "error" in payload:
            message = str(payload.get("error", "Internal adapter execution failed."))
            raise self._map_backend_error(
                message=message,
                payload=payload,
                returncode=process.returncode,
            )

        files = payload.get("files")
        if not isinstance(files, list):
            raise BackendToolError(
                code=ErrorCode.BACKEND_INVALID_RESPONSE,
                message="Internal adapter response does not include a valid files array.",
                retryable=True,
                suggestion="Inspect the adapter contract and normalize its output.",
                details={},
            )

        normalized_files = [
            self._normalize_relative_path(workspace_root, str(item).strip())
            for item in files
            if str(item).strip()
        ]

        return InternalTraceSymbolResult(
            files=normalized_files,
            count=int(payload.get("count", len(normalized_files))),
            options=payload.get("options", {}),
        )

    def _map_backend_error(
        self,
        *,
        message: str,
        payload: dict[str, Any],
        returncode: int,
    ) -> BackendToolError:
        details = {"returncode": returncode}

        if message.startswith("File not found:"):
            return BackendToolError(
                code=ErrorCode.FILE_NOT_FOUND,
                message=message,
                retryable=False,
                suggestion="Provide an existing file path inside the workspace root.",
                details=details,
            )

        if message.startswith("Symbol '") and " was not found in '" in message:
            return BackendToolError(
                code=ErrorCode.SYMBOL_NOT_FOUND,
                message=message,
                retryable=False,
                suggestion="Verify the symbol name and starting file, then retry.",
                details=details,
            )

        if message.startswith("Unsupported or unreadable file:"):
            return BackendToolError(
                code=ErrorCode.UNSUPPORTED_FILE,
                message=message,
                retryable=False,
                suggestion="Use a readable Java or TypeScript source file supported by the internal analyzer.",
                details=details,
            )

        if "tree-sitter parser not available" in message:
            return BackendToolError(
                code=ErrorCode.BACKEND_DEPENDENCY_NOT_FOUND,
                message=message,
                retryable=False,
                suggestion="Install the required tree-sitter parser for the requested language and retry.",
                details=details,
            )

        return BackendToolError(
            code=ErrorCode.BACKEND_EXECUTION_FAILED,
            message=message,
            retryable=True,
            suggestion="Verify tree-sitter dependencies and internal analyzer availability.",
            details=details,
        )

    def _normalize_relative_path(self, workspace_root: Path, path_value: str) -> str:
        candidate = Path(path_value)
        if candidate.is_absolute():
            try:
                return candidate.relative_to(workspace_root).as_posix()
            except ValueError:
                return candidate.as_posix()
        return candidate.as_posix()
