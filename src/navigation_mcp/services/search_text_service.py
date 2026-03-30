from __future__ import annotations

from pathlib import Path
from typing import Any

from pydantic import ValidationError

from navigation_mcp.adapters.internal_tools import (
    BackendToolError,
    InternalSearchTextAdapter,
)
from navigation_mcp.contracts.code import (
    PublicFramework,
    PublicLanguage,
    SearchTextData,
    SearchTextFileMatch,
    SearchTextInput,
    SearchTextMatch,
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
    resolve_effective_language,
    resolve_optional_scope,
)

TOOL_NAME = "code.search_text"


class SearchTextService:
    def __init__(
        self, *, workspace_root: Path, adapter: InternalSearchTextAdapter
    ) -> None:
        self.workspace_root = workspace_root.resolve()
        self.adapter = adapter

    def execute(self, request: SearchTextInput) -> ResponseEnvelope[SearchTextData]:
        path_scope = resolve_optional_scope(self.workspace_root, request.path)
        path_filter = path_scope.relative if path_scope is not None else None
        language_globs = self._resolve_language_globs(
            request.language, request.framework
        )
        internal = self.adapter.search_text(
            workspace_root=self.workspace_root,
            query=request.query,
            path_filter=path_filter,
            include=request.include,
            regex=request.regex,
            context=request.context,
            language_globs=language_globs,
        )

        normalized_items = [self._normalize_file(item) for item in internal.files]
        total_files = len(normalized_items)
        truncated = total_files > request.limit
        items = normalized_items[: request.limit]

        file_count = len(items) if truncated else total_files
        match_count = sum(item.matchCount for item in items)

        errors: list[ErrorItem] = []
        status = ResponseStatus.OK
        if truncated:
            status = ResponseStatus.PARTIAL
            errors.append(
                ErrorItem(
                    code=ErrorCode.RESULT_TRUNCATED,
                    message=f"Result set exceeded the requested limit of {request.limit} files.",
                    retryable=False,
                    suggestion="Increase limit or narrow the path/include/language filters.",
                    details={
                        "returnedFiles": len(items),
                        "totalFiles": total_files,
                        "returnedMatches": match_count,
                        "totalMatches": internal.match_count,
                        "limit": request.limit,
                    },
                )
            )

        return ResponseEnvelope[SearchTextData](
            tool=TOOL_NAME,
            status=status,
            summary=self._build_summary(
                query=request.query,
                file_count=total_files,
                match_count=internal.match_count,
                truncated=truncated,
            ),
            data=SearchTextData(
                fileCount=file_count,
                matchCount=match_count if truncated else internal.match_count,
                totalFileCount=total_files,
                totalMatchCount=internal.match_count,
                items=items,
            ),
            errors=errors,
            meta=build_response_meta(
                query=request.model_dump(mode="json"),
                resolved_path=path_scope.public_path
                if path_scope is not None
                else None,
                truncated=truncated,
                counts={
                    "returnedFileCount": len(items),
                    "totalFileCount": total_files,
                    "returnedMatchCount": match_count,
                    "totalMatchCount": internal.match_count,
                },
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
    ) -> ResponseEnvelope[SearchTextData]:
        details = []
        for item in error.errors():
            details.append(
                {
                    "field": ".".join(str(part) for part in item.get("loc", [])),
                    "message": item.get("msg", "Invalid value."),
                    "type": item.get("type", "validation_error"),
                }
            )

        return ResponseEnvelope[SearchTextData](
            tool=TOOL_NAME,
            status=ResponseStatus.ERROR,
            summary="Request validation failed.",
            data=SearchTextData(
                fileCount=0,
                matchCount=0,
                totalFileCount=0,
                totalMatchCount=0,
                items=[],
            ),
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
        request: SearchTextInput,
        error: BackendToolError,
    ) -> ResponseEnvelope[SearchTextData]:
        return ResponseEnvelope[SearchTextData](
            tool=TOOL_NAME,
            status=ResponseStatus.ERROR,
            summary="Text search failed.",
            data=SearchTextData(
                fileCount=0,
                matchCount=0,
                totalFileCount=0,
                totalMatchCount=0,
                items=[],
            ),
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
    ) -> ResponseEnvelope[SearchTextData]:
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

        return ResponseEnvelope[SearchTextData](
            tool=TOOL_NAME,
            status=ResponseStatus.ERROR,
            summary=summary,
            data=SearchTextData(
                fileCount=0,
                matchCount=0,
                totalFileCount=0,
                totalMatchCount=0,
                items=[],
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

    def _resolve_language_globs(
        self,
        language: PublicLanguage | None,
        framework: PublicFramework | None,
    ) -> list[str]:
        effective = resolve_effective_language(language, framework)
        if effective == PublicLanguage.TYPESCRIPT:
            return ["*.ts", "*.tsx"]
        if effective == PublicLanguage.JAVASCRIPT:
            return ["*.js", "*.jsx"]
        if effective == PublicLanguage.JAVA:
            return ["*.java"]
        return []

    def _normalize_file(self, item: dict[str, Any]) -> SearchTextFileMatch:
        path = str(item.get("path", "")).strip()
        return SearchTextFileMatch(
            path=path,
            language=infer_language_from_path(path),
            matchCount=int(item.get("matchCount", 0)),
            matches=[
                SearchTextMatch.model_validate(match)
                for match in item.get("matches", [])
            ],
        )

    def _build_summary(
        self,
        *,
        query: str,
        file_count: int,
        match_count: int,
        truncated: bool,
    ) -> str:
        if match_count == 0:
            return f"No text matches found for '{query}'."
        if truncated:
            return (
                f"Found {match_count} text matches across {file_count} files for '{query}' "
                "and returned a truncated subset."
            )
        if match_count == 1:
            return f"Found 1 text match in 1 file for '{query}'."
        return (
            f"Found {match_count} text matches across {file_count} files for '{query}'."
        )
