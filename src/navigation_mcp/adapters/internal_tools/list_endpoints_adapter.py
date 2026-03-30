from __future__ import annotations

import json
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from navigation_mcp.adapters.internal_tools.find_symbol_adapter import BackendToolError
from navigation_mcp.contracts.common import ErrorCode

DEFAULT_LIST_ENDPOINTS_SCRIPT = Path(
    "/home/j0k3r/.config/opencode/tools/list_endpoints.py"
)


@dataclass(slots=True)
class InternalListEndpointsResult:
    by_language: list[dict[str, Any]]
    errors: list[dict[str, Any]]
    options: dict[str, Any]


class InternalListEndpointsAdapter:
    def __init__(self, script_path: Path | None = None) -> None:
        self.script_path = (
            (script_path or DEFAULT_LIST_ENDPOINTS_SCRIPT).expanduser().resolve()
        )

    def list_endpoints(
        self,
        *,
        workspace_root: Path,
        options: dict[str, Any],
    ) -> InternalListEndpointsResult:
        if not self.script_path.exists():
            raise BackendToolError(
                code=ErrorCode.BACKEND_SCRIPT_NOT_FOUND,
                message="Internal endpoint analyzer is not available.",
                retryable=False,
                suggestion="Install or configure the internal endpoint analyzer and retry.",
            )

        process = subprocess.run(
            [
                sys.executable,
                str(self.script_path),
                str(workspace_root),
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

        by_language = payload.get("byLanguage")
        if not isinstance(by_language, list):
            raise BackendToolError(
                code=ErrorCode.BACKEND_INVALID_RESPONSE,
                message="Internal adapter response does not include a valid byLanguage array.",
                retryable=True,
                suggestion="Inspect the adapter contract and normalize its output.",
                details={},
            )

        errors = payload.get("errors", [])
        normalized_errors = errors if isinstance(errors, list) else []

        return InternalListEndpointsResult(
            by_language=[item for item in by_language if isinstance(item, dict)],
            errors=[item for item in normalized_errors if isinstance(item, dict)],
            options=options,
        )

    def _map_backend_error(
        self,
        *,
        message: str,
        payload: dict[str, Any],
        returncode: int,
    ) -> BackendToolError:
        details = {"returncode": returncode}

        if message.startswith("Workspace not found:"):
            return BackendToolError(
                code=ErrorCode.FILE_NOT_FOUND,
                message=message,
                retryable=False,
                suggestion="Provide an existing workspace path or verify the configured workspace root.",
                details=details,
            )

        if "tree-sitter-java not available" in message:
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
            suggestion="Verify internal analyzer availability and retry.",
            details=details,
        )
