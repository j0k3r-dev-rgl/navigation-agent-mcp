from __future__ import annotations

from pathlib import Path
from typing import Any

from pydantic import ValidationError

from navigation_mcp.adapters.internal_tools import (
    BackendToolError,
    InternalTraceSymbolAdapter,
)
from navigation_mcp.contracts.code import (
    PublicFramework,
    PublicLanguage,
    TraceSymbolData,
    TraceSymbolEntrypoint,
    TraceSymbolFile,
    TraceSymbolInput,
)
from navigation_mcp.contracts.common import (
    ErrorCode,
    ErrorItem,
    ResponseEnvelope,
    ResponseStatus,
)
from navigation_mcp.services.shared import (
    build_response_meta,
    infer_language_from_path,
    resolve_backend_language,
    resolve_effective_language,
    resolve_required_scope,
)

TOOL_NAME = "code.trace_symbol"


class TraceSymbolService:
    def __init__(
        self, *, workspace_root: Path, adapter: InternalTraceSymbolAdapter
    ) -> None:
        self.workspace_root = workspace_root.resolve()
        self.adapter = adapter

    def execute(self, request: TraceSymbolInput) -> ResponseEnvelope[TraceSymbolData]:
        start_scope = resolve_required_scope(self.workspace_root, request.path)
        start_path = start_scope.relative or Path(".")
        backend_language = resolve_backend_language(request.language, request.framework)
        internal = self.adapter.trace_symbol(
            workspace_root=self.workspace_root,
            path=start_path,
            symbol=request.symbol,
            options={"language": backend_language},
        )

        items = self._normalize_files(internal.files)
        entrypoint = TraceSymbolEntrypoint(
            path=start_path.as_posix(),
            symbol=request.symbol,
            language=infer_language_from_path(start_path.as_posix()),
        )

        return ResponseEnvelope[TraceSymbolData](
            tool=TOOL_NAME,
            status=ResponseStatus.OK,
            summary=self._build_summary(
                symbol=request.symbol,
                path=start_path.as_posix(),
                file_count=len(items),
            ),
            data=TraceSymbolData(
                entrypoint=entrypoint,
                fileCount=len(items),
                items=items,
            ),
            errors=[],
            meta=build_response_meta(
                query=request.model_dump(mode="json"),
                resolved_path=start_scope.public_path,
                counts={"returnedCount": len(items), "totalMatched": len(items)},
                detection={
                    "effectiveLanguage": (
                        resolve_effective_language(
                            request.language, request.framework
                        ).value
                        if resolve_effective_language(
                            request.language, request.framework
                        )
                        is not None
                        else None
                    ),
                    "framework": request.framework.value if request.framework else None,
                },
            ),
        )

    def error_from_validation(
        self, error: ValidationError
    ) -> ResponseEnvelope[TraceSymbolData]:
        details = []
        for item in error.errors():
            details.append(
                {
                    "field": ".".join(str(part) for part in item.get("loc", [])),
                    "message": item.get("msg", "Invalid value."),
                    "type": item.get("type", "validation_error"),
                }
            )

        return ResponseEnvelope[TraceSymbolData](
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

    def error_from_backend(
        self,
        *,
        request: TraceSymbolInput,
        error: BackendToolError,
    ) -> ResponseEnvelope[TraceSymbolData]:
        return ResponseEnvelope[TraceSymbolData](
            tool=TOOL_NAME,
            status=ResponseStatus.ERROR,
            summary="Symbol trace failed.",
            data=self._empty_data(path=request.path, symbol=request.symbol),
            errors=[
                ErrorItem(
                    code=error.code,
                    message=error.message,
                    retryable=error.retryable,
                    suggestion=error.suggestion,
                    details=error.details,
                )
            ],
            meta=build_response_meta(query=request.model_dump(mode="json")),
        )

    def error_from_path(
        self,
        *,
        request_payload: dict[str, Any],
        path_value: str,
        exists: bool,
    ) -> ResponseEnvelope[TraceSymbolData]:
        if exists:
            code = ErrorCode.PATH_OUTSIDE_WORKSPACE
            message = f"Path '{path_value}' is outside the configured workspace root."
            suggestion = "Use a file path inside the workspace root."
            summary = "Path validation failed."
        else:
            code = ErrorCode.FILE_NOT_FOUND
            message = f"Path '{path_value}' was not found inside the configured workspace root."
            suggestion = "Provide an existing file path inside the workspace root."
            summary = "Path not found."

        return ResponseEnvelope[TraceSymbolData](
            tool=TOOL_NAME,
            status=ResponseStatus.ERROR,
            summary=summary,
            data=self._empty_data(
                path=str(request_payload.get("path") or ""),
                symbol=str(request_payload.get("symbol") or ""),
            ),
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

    def _normalize_files(self, files: list[str]) -> list[TraceSymbolFile]:
        unique_paths = sorted({path for path in files if path})
        return [
            TraceSymbolFile(
                path=path,
                language=infer_language_from_path(path),
            )
            for path in unique_paths
        ]

    def _build_summary(self, *, symbol: str, path: str, file_count: int) -> str:
        if file_count == 0:
            return f"Trace completed for '{symbol}' from '{path}' with no related files found."
        if file_count == 1:
            return f"Traced 1 related file for '{symbol}' from '{path}'."
        return f"Traced {file_count} related files for '{symbol}' from '{path}'."

    def _empty_data(self, *, path: str = "", symbol: str = "") -> TraceSymbolData:
        return TraceSymbolData(
            entrypoint=TraceSymbolEntrypoint(
                path=path,
                symbol=symbol,
                language=infer_language_from_path(path),
            ),
            fileCount=0,
            items=[],
        )
