from __future__ import annotations

from pathlib import Path
from typing import Any

from pydantic import ValidationError

from navigation_mcp.adapters.internal_tools import (
    BackendToolError,
    InternalFindSymbolAdapter,
)
from navigation_mcp.contracts.code import (
    FindSymbolData,
    FindSymbolInput,
    MatchMode,
    PublicFramework,
    PublicLanguage,
    PublicSymbolDefinition,
    normalize_public_symbol_kind,
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
    resolve_optional_scope,
)

TOOL_NAME = "code.find_symbol"


class FindSymbolService:
    def __init__(
        self, *, workspace_root: Path, adapter: InternalFindSymbolAdapter
    ) -> None:
        self.workspace_root = workspace_root.resolve()
        self.adapter = adapter

    def execute(self, request: FindSymbolInput) -> ResponseEnvelope[FindSymbolData]:
        path_scope = resolve_optional_scope(self.workspace_root, request.path)
        path_filter = path_scope.relative if path_scope is not None else None
        backend_language = resolve_backend_language(request.language, request.framework)
        options = {
            "language": backend_language,
            "kind": request.kind.value,
            "fuzzy": request.match is MatchMode.FUZZY,
        }

        internal = self.adapter.find_symbol(
            workspace_root=self.workspace_root,
            symbol=request.symbol,
            options=options,
        )

        normalized = [self._normalize_item(item) for item in internal.matches]
        filtered = self._filter_results(
            items=normalized,
            language=request.language,
            path_filter=path_filter,
        )

        total_count = len(filtered)
        truncated = total_count > request.limit
        items = filtered[: request.limit]

        errors: list[ErrorItem] = []
        status = ResponseStatus.OK
        if truncated:
            status = ResponseStatus.PARTIAL
            errors.append(
                ErrorItem(
                    code=ErrorCode.RESULT_TRUNCATED,
                    message=f"Result set exceeded the requested limit of {request.limit} items.",
                    retryable=False,
                    suggestion="Increase limit or narrow the path/language filter.",
                    details={
                        "returned": len(items),
                        "total": total_count,
                        "limit": request.limit,
                    },
                )
            )

        summary = self._build_summary(
            symbol=request.symbol, count=total_count, truncated=truncated
        )
        data = FindSymbolData(
            count=total_count,
            returnedCount=len(items),
            totalMatched=total_count,
            items=items,
        )

        return ResponseEnvelope[FindSymbolData](
            tool=TOOL_NAME,
            status=status,
            summary=summary,
            data=data,
            errors=errors,
            meta=build_response_meta(
                query=request.model_dump(mode="json"),
                resolved_path=path_scope.public_path
                if path_scope is not None
                else None,
                truncated=truncated,
                counts={"returnedCount": len(items), "totalMatched": total_count},
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
    ) -> ResponseEnvelope[FindSymbolData]:
        details = []
        for item in error.errors():
            details.append(
                {
                    "field": ".".join(str(part) for part in item.get("loc", [])),
                    "message": item.get("msg", "Invalid value."),
                    "type": item.get("type", "validation_error"),
                }
            )

        return ResponseEnvelope[FindSymbolData](
            tool=TOOL_NAME,
            status=ResponseStatus.ERROR,
            summary="Request validation failed.",
            data=FindSymbolData(count=0, returnedCount=0, totalMatched=0, items=[]),
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
        request: FindSymbolInput,
        error: BackendToolError,
    ) -> ResponseEnvelope[FindSymbolData]:
        return ResponseEnvelope[FindSymbolData](
            tool=TOOL_NAME,
            status=ResponseStatus.ERROR,
            summary="Symbol analysis failed.",
            data=FindSymbolData(count=0, returnedCount=0, totalMatched=0, items=[]),
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
    ) -> ResponseEnvelope[FindSymbolData]:
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

        return ResponseEnvelope[FindSymbolData](
            tool=TOOL_NAME,
            status=ResponseStatus.ERROR,
            summary=summary,
            data=FindSymbolData(count=0, returnedCount=0, totalMatched=0, items=[]),
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

    def _normalize_item(self, item: dict[str, Any]) -> PublicSymbolDefinition:
        raw_path = str(item.get("file", "")).strip()
        path = self._normalize_relative_path(raw_path)
        return PublicSymbolDefinition(
            symbol=str(item.get("name", "")).strip(),
            kind=normalize_public_symbol_kind(item.get("kind")),
            path=path,
            line=int(item.get("line", 1)),
            language=infer_language_from_path(path),
        )

    def _filter_results(
        self,
        *,
        items: list[PublicSymbolDefinition],
        language: PublicLanguage | None,
        path_filter: Path | None,
    ) -> list[PublicSymbolDefinition]:
        filtered = items

        if language is not None:
            filtered = [item for item in filtered if item.language == language]

        if path_filter is not None:
            prefix = path_filter.as_posix()
            filtered = [
                item
                for item in filtered
                if item.path == prefix or item.path.startswith(f"{prefix}/")
            ]

        return sorted(
            filtered, key=lambda item: (item.path, item.line, item.symbol, item.kind)
        )

    def _normalize_relative_path(self, path_value: str) -> str:
        candidate = Path(path_value)
        if candidate.is_absolute():
            try:
                return candidate.relative_to(self.workspace_root).as_posix()
            except ValueError:
                return candidate.as_posix()
        return candidate.as_posix()

    def _build_summary(self, *, symbol: str, count: int, truncated: bool) -> str:
        if count == 0:
            return f"No symbol definitions found for '{symbol}'."
        if truncated:
            return f"Found {count} symbol definitions for '{symbol}' and returned a truncated subset."
        if count == 1:
            return f"Found 1 symbol definition for '{symbol}'."
        return f"Found {count} symbol definitions for '{symbol}'."
