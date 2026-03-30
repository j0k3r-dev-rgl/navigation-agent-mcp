from __future__ import annotations

import json
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from navigation_mcp.adapters.internal_tools.find_symbol_adapter import BackendToolError
from navigation_mcp.contracts.common import ErrorCode

DEFAULT_TRACE_CALLERS_SCRIPT = Path(
    "/home/j0k3r/.config/opencode/tools/trace_callers.py"
)


@dataclass(slots=True)
class InternalTraceCallersResult:
    target: dict[str, Any]
    callers: list[dict[str, Any]]
    count: int
    options: dict[str, Any]
    mode: str | None
    direct_summary: dict[str, Any]
    recursive_result: dict[str, Any] | None


class InternalTraceCallersAdapter:
    def __init__(self, script_path: Path | None = None) -> None:
        self.script_path = (
            (script_path or DEFAULT_TRACE_CALLERS_SCRIPT).expanduser().resolve()
        )

    def trace_callers(
        self,
        *,
        workspace_root: Path,
        path: Path,
        symbol: str,
        options: dict[str, Any],
    ) -> InternalTraceCallersResult:
        if not self.script_path.exists():
            raise BackendToolError(
                code=ErrorCode.BACKEND_SCRIPT_NOT_FOUND,
                message="Internal caller-trace analyzer is not available.",
                retryable=False,
                suggestion="Install or configure the internal caller-trace analyzer and retry.",
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

        callers = payload.get("callers")
        if not isinstance(callers, list):
            raise BackendToolError(
                code=ErrorCode.BACKEND_INVALID_RESPONSE,
                message="Internal adapter response does not include a valid callers array.",
                retryable=True,
                suggestion="Inspect the adapter contract and normalize its output.",
                details={},
            )

        target = payload.get("target")
        if not isinstance(target, dict):
            raise BackendToolError(
                code=ErrorCode.BACKEND_INVALID_RESPONSE,
                message="Internal adapter response does not include a valid target object.",
                retryable=True,
                suggestion="Inspect the adapter contract and normalize its output.",
                details={},
            )

        normalized_callers = [
            {
                **item,
                "file": self._normalize_relative_path(
                    workspace_root, str(item.get("file", "")).strip()
                ),
            }
            for item in callers
            if isinstance(item, dict)
        ]

        normalized_target = {
            **target,
            "file": self._normalize_relative_path(
                workspace_root, str(target.get("file", "")).strip()
            ),
        }

        recursive_result = payload.get("recursiveResult")
        normalized_recursive = (
            self._normalize_recursive_result(workspace_root, recursive_result)
            if isinstance(recursive_result, dict)
            else None
        )

        return InternalTraceCallersResult(
            target=normalized_target,
            callers=normalized_callers,
            count=int(payload.get("count", len(normalized_callers))),
            options=payload.get("options", {}),
            mode=str(payload.get("mode")) if payload.get("mode") is not None else None,
            direct_summary=payload.get("directSummary", {}),
            recursive_result=normalized_recursive,
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

    def _normalize_recursive_result(
        self, workspace_root: Path, result: dict[str, Any]
    ) -> dict[str, Any]:
        normalized = dict(result)

        root = normalized.get("root")
        if isinstance(root, dict):
            normalized["root"] = self._normalize_recursive_node(workspace_root, root)

        nodes = normalized.get("nodes")
        if isinstance(nodes, list):
            normalized["nodes"] = [
                self._normalize_recursive_node(workspace_root, item)
                for item in nodes
                if isinstance(item, dict)
            ]

        truncated = normalized.get("truncated")
        if isinstance(truncated, list):
            normalized["truncated"] = [
                {
                    **item,
                    "file": self._normalize_relative_path(
                        workspace_root, str(item.get("file", "")).strip()
                    ),
                }
                for item in truncated
                if isinstance(item, dict)
            ]

        paths = normalized.get("paths")
        if isinstance(paths, list):
            normalized["paths"] = [
                [
                    {
                        **segment,
                        "file": self._normalize_relative_path(
                            workspace_root, str(segment.get("file", "")).strip()
                        ),
                    }
                    for segment in path
                    if isinstance(segment, dict)
                ]
                for path in paths
                if isinstance(path, list)
            ]

        probable = normalized.get("probableEntryPoints")
        if isinstance(probable, list):
            normalized["probableEntryPoints"] = [
                self._normalize_probable_entry_point(workspace_root, item)
                for item in probable
                if isinstance(item, dict)
            ]

        classifications = normalized.get("classifications")
        if isinstance(classifications, dict):
            normalized["classifications"] = self._normalize_classifications(
                workspace_root, classifications
            )

        return normalized

    def _normalize_recursive_node(
        self, workspace_root: Path, node: dict[str, Any]
    ) -> dict[str, Any]:
        normalized = dict(node)
        normalized["file"] = self._normalize_relative_path(
            workspace_root, str(node.get("file", "")).strip()
        )
        return normalized

    def _normalize_probable_entry_point(
        self, workspace_root: Path, item: dict[str, Any]
    ) -> dict[str, Any]:
        normalized = dict(item)
        normalized["file"] = self._normalize_relative_path(
            workspace_root, str(item.get("file", "")).strip()
        )
        path_from_target = normalized.get("pathFromTarget")
        if isinstance(path_from_target, list):
            normalized["pathFromTarget"] = [
                {
                    **segment,
                    "file": self._normalize_relative_path(
                        workspace_root, str(segment.get("file", "")).strip()
                    ),
                }
                for segment in path_from_target
                if isinstance(segment, dict)
            ]
        return normalized

    def _normalize_classifications(
        self, workspace_root: Path, classifications: dict[str, Any]
    ) -> dict[str, Any]:
        normalized = dict(classifications)
        for key in ("directCallers", "indirectCallers"):
            items = normalized.get(key)
            if isinstance(items, list):
                normalized[key] = [
                    self._normalize_classification_record(workspace_root, item)
                    for item in items
                    if isinstance(item, dict)
                ]

        probable_public = normalized.get("probablePublicEntryPoints")
        if isinstance(probable_public, list):
            normalized["probablePublicEntryPoints"] = [
                self._normalize_probable_entry_point(workspace_root, item)
                for item in probable_public
                if isinstance(item, dict)
            ]

        chains = normalized.get("implementationInterfaceChain")
        if isinstance(chains, list):
            normalized["implementationInterfaceChain"] = [
                self._normalize_interface_chain(workspace_root, item)
                for item in chains
                if isinstance(item, dict)
            ]

        return normalized

    def _normalize_classification_record(
        self, workspace_root: Path, item: dict[str, Any]
    ) -> dict[str, Any]:
        normalized = dict(item)
        normalized["file"] = self._normalize_relative_path(
            workspace_root, str(item.get("file", "")).strip()
        )
        calls = normalized.get("calls")
        if isinstance(calls, dict):
            normalized["calls"] = {
                **calls,
                "file": self._normalize_relative_path(
                    workspace_root, str(calls.get("file", "")).strip()
                ),
            }
        path_from_target = normalized.get("pathFromTarget")
        if isinstance(path_from_target, list):
            normalized["pathFromTarget"] = [
                {
                    **segment,
                    "file": self._normalize_relative_path(
                        workspace_root, str(segment.get("file", "")).strip()
                    ),
                }
                for segment in path_from_target
                if isinstance(segment, dict)
            ]
        return normalized

    def _normalize_interface_chain(
        self, workspace_root: Path, item: dict[str, Any]
    ) -> dict[str, Any]:
        normalized = dict(item)

        interface = normalized.get("interface")
        if isinstance(interface, dict) and interface.get("file") is not None:
            normalized["interface"] = {
                **interface,
                "file": self._normalize_relative_path(
                    workspace_root, str(interface.get("file", "")).strip()
                ),
            }

        implementation = normalized.get("implementation")
        if isinstance(implementation, dict):
            normalized["implementation"] = {
                **implementation,
                "file": self._normalize_relative_path(
                    workspace_root, str(implementation.get("file", "")).strip()
                ),
            }

        implementations = normalized.get("implementations")
        if isinstance(implementations, list):
            normalized["implementations"] = [
                {
                    **entry,
                    "file": self._normalize_relative_path(
                        workspace_root, str(entry.get("file", "")).strip()
                    ),
                }
                for entry in implementations
                if isinstance(entry, dict)
            ]

        callers = normalized.get("callers")
        if isinstance(callers, list):
            normalized["callers"] = [
                self._normalize_classification_record(workspace_root, caller)
                for caller in callers
                if isinstance(caller, dict)
            ]

        return normalized

    def _normalize_relative_path(self, workspace_root: Path, path_value: str) -> str:
        candidate = Path(path_value)
        if candidate.is_absolute():
            try:
                return candidate.relative_to(workspace_root).as_posix()
            except ValueError:
                return candidate.as_posix()
        return candidate.as_posix()
