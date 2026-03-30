from __future__ import annotations

import json
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from navigation_mcp.contracts.common import ErrorCode

DEFAULT_FIND_SYMBOL_SCRIPT = Path("/home/j0k3r/.config/opencode/tools/find_symbol.py")


class BackendToolError(Exception):
    def __init__(
        self,
        *,
        code: ErrorCode | str,
        message: str,
        retryable: bool,
        suggestion: str | None = None,
        details: dict[str, Any] | None = None,
    ) -> None:
        super().__init__(message)
        self.code = code
        self.message = message
        self.retryable = retryable
        self.suggestion = suggestion
        self.details = details or {}


@dataclass(slots=True)
class InternalFindSymbolResult:
    matches: list[dict[str, Any]]
    count: int
    options: dict[str, Any]


class InternalFindSymbolAdapter:
    def __init__(self, script_path: Path | None = None) -> None:
        self.script_path = (
            (script_path or DEFAULT_FIND_SYMBOL_SCRIPT).expanduser().resolve()
        )

    def find_symbol(
        self,
        *,
        workspace_root: Path,
        symbol: str,
        options: dict[str, Any],
    ) -> InternalFindSymbolResult:
        if not self.script_path.exists():
            raise BackendToolError(
                code=ErrorCode.BACKEND_SCRIPT_NOT_FOUND,
                message="Internal symbol analyzer is not available.",
                retryable=False,
                suggestion="Install or configure the internal symbol analyzer and retry.",
            )

        process = subprocess.run(
            [
                sys.executable,
                str(self.script_path),
                str(workspace_root),
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
            raise BackendToolError(
                code=ErrorCode.BACKEND_EXECUTION_FAILED,
                message=str(payload.get("error", "Internal adapter execution failed.")),
                retryable=True,
                suggestion="Verify tree-sitter dependencies and internal analyzer availability.",
                details={"returncode": process.returncode},
            )

        matches = payload.get("matches")
        if not isinstance(matches, list):
            raise BackendToolError(
                code=ErrorCode.BACKEND_INVALID_RESPONSE,
                message="Internal adapter response does not include a valid matches array.",
                retryable=True,
                suggestion="Inspect the adapter contract and normalize its output.",
                details={},
            )

        return InternalFindSymbolResult(
            matches=matches,
            count=int(payload.get("count", len(matches))),
            options=payload.get("options", {}),
        )
