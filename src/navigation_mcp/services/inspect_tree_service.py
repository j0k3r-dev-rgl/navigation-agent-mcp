from __future__ import annotations

from pathlib import Path
from typing import Any

from pydantic import ValidationError

from navigation_mcp.adapters.internal_tools import InternalInspectTreeAdapter
from navigation_mcp.contracts.code import (
    InspectTreeData,
    InspectTreeInput,
    InspectTreeItem,
    InspectTreeItemStats,
)
from navigation_mcp.contracts.common import (
    ErrorCode,
    ErrorItem,
    ResponseEnvelope,
    ResponseStatus,
)
from navigation_mcp.services.shared import build_response_meta, resolve_optional_scope

TOOL_NAME = "code.inspect_tree"


class InspectTreeService:
    def __init__(
        self, *, workspace_root: Path, adapter: InternalInspectTreeAdapter
    ) -> None:
        self.workspace_root = workspace_root.resolve()
        self.adapter = adapter

    def execute(self, request: InspectTreeInput) -> ResponseEnvelope[InspectTreeData]:
        root_scope = resolve_optional_scope(self.workspace_root, request.path)
        root_path = (
            root_scope.absolute if root_scope is not None else self.workspace_root
        )

        internal = self.adapter.inspect_tree(
            workspace_root=self.workspace_root,
            root_path=root_path,
            max_depth=request.max_depth,
            extensions=request.extensions,
            file_pattern=request.file_pattern,
            include_stats=request.include_stats,
            include_hidden=request.include_hidden,
        )

        items = [self._normalize_item(item) for item in internal.items]
        errors: list[ErrorItem] = []
        status = ResponseStatus.OK
        if internal.truncated:
            status = ResponseStatus.PARTIAL
            errors.append(
                ErrorItem(
                    code=ErrorCode.RESULT_TRUNCATED,
                    message=f"Tree inspection hit the safety cap of {internal.max_items} items.",
                    retryable=False,
                    suggestion="Narrow the path, reduce max_depth, or add extensions/file_pattern filters.",
                    details={
                        "returned": len(items),
                        "maxItems": internal.max_items,
                    },
                )
            )

        return ResponseEnvelope[InspectTreeData](
            tool=TOOL_NAME,
            status=status,
            summary=self._build_summary(
                root=internal.root,
                entry_count=len(items),
                truncated=internal.truncated,
            ),
            data=InspectTreeData(
                root=internal.root,
                entryCount=len(items),
                items=items,
            ),
            errors=errors,
            meta=build_response_meta(
                query=request.model_dump(mode="json"),
                resolved_path=internal.root,
                truncated=internal.truncated,
                counts={
                    "returnedCount": len(items),
                    "totalMatched": len(items) if not internal.truncated else None,
                },
                detection={
                    "includeHidden": "true" if request.include_hidden else "false",
                    "stats": "true" if request.include_stats else "false",
                },
            ),
        )

    def error_from_validation(
        self, error: ValidationError
    ) -> ResponseEnvelope[InspectTreeData]:
        details = []
        for item in error.errors():
            details.append(
                {
                    "field": ".".join(str(part) for part in item.get("loc", [])),
                    "message": item.get("msg", "Invalid value."),
                    "type": item.get("type", "validation_error"),
                }
            )

        return ResponseEnvelope[InspectTreeData](
            tool=TOOL_NAME,
            status=ResponseStatus.ERROR,
            summary="Request validation failed.",
            data=self._empty_data(),
            errors=[
                ErrorItem(
                    code=ErrorCode.INVALID_INPUT,
                    message="One or more input fields are invalid.",
                    retryable=False,
                    suggestion="Correct the invalid fields and try again.",
                    details={"issues": details},
                )
            ],
            meta=build_response_meta(query={}),
        )

    def error_from_path(
        self,
        *,
        request_payload: dict[str, Any],
        path_value: str,
        exists: bool,
    ) -> ResponseEnvelope[InspectTreeData]:
        if exists:
            code = ErrorCode.PATH_OUTSIDE_WORKSPACE
            message = f"Path '{path_value}' is outside the configured workspace root."
            suggestion = "Use a path inside the workspace root or omit the path filter."
            summary = "Path validation failed."
        else:
            code = ErrorCode.FILE_NOT_FOUND
            message = f"Path '{path_value}' was not found inside the configured workspace root."
            suggestion = (
                "Provide an existing file or directory path inside the workspace root."
            )
            summary = "Path not found."

        return ResponseEnvelope[InspectTreeData](
            tool=TOOL_NAME,
            status=ResponseStatus.ERROR,
            summary=summary,
            data=self._empty_data(),
            errors=[
                ErrorItem(
                    code=code,
                    message=message,
                    retryable=False,
                    suggestion=suggestion,
                    details={"path": path_value},
                )
            ],
            meta=build_response_meta(query=request_payload),
        )

    def _normalize_item(self, item: dict[str, Any]) -> InspectTreeItem:
        stats_payload = item.get("stats")
        return InspectTreeItem(
            path=str(item.get("path", "")).strip(),
            name=str(item.get("name", "")).strip(),
            type=str(item.get("type", "file")).strip(),
            depth=int(item.get("depth", 1)),
            extension=item.get("extension"),
            stats=(
                InspectTreeItemStats.model_validate(stats_payload)
                if isinstance(stats_payload, dict)
                else None
            ),
        )

    def _build_summary(self, *, root: str, entry_count: int, truncated: bool) -> str:
        if entry_count == 0:
            return f"No tree entries found under '{root}'."
        if truncated:
            return f"Inspected {entry_count} tree entries under '{root}' and returned a truncated subset."
        if entry_count == 1:
            return f"Inspected 1 tree entry under '{root}'."
        return f"Inspected {entry_count} tree entries under '{root}'."

    def _empty_data(self) -> InspectTreeData:
        return InspectTreeData(root=".", entryCount=0, items=[])
