from __future__ import annotations

from pathlib import Path
import re
from typing import Any

from pydantic import ValidationError

from navigation_mcp.adapters.internal_tools import (
    BackendToolError,
    InternalListEndpointsAdapter,
)
from navigation_mcp.contracts.code import (
    ListEndpointsData,
    ListEndpointsGroupedCounts,
    ListEndpointsInput,
    PublicEndpointItem,
    PublicEndpointKind,
    PublicFramework,
    PublicLanguage,
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

TOOL_NAME = "code.list_endpoints"

GRAPHQL_KIND_TOKENS = {
    "graphql",
    "query",
    "queries",
    "mutation",
    "mutations",
}

REST_KIND_TOKENS = {
    "rest",
    "http",
    "request",
    "requests",
    "get",
    "post",
    "put",
    "patch",
    "delete",
    "head",
    "options",
}

ROUTE_KIND_TOKENS = {
    "route",
    "routes",
    "react_router",
    "loader",
    "loaders",
    "action",
    "actions",
    "layout",
    "layouts",
    "resource",
    "resources",
    "component",
    "components",
    "page",
    "pages",
}


class ListEndpointsService:
    def __init__(
        self, *, workspace_root: Path, adapter: InternalListEndpointsAdapter
    ) -> None:
        self.workspace_root = workspace_root.resolve()
        self.adapter = adapter

    def execute(
        self, request: ListEndpointsInput
    ) -> ResponseEnvelope[ListEndpointsData]:
        path_scope = resolve_optional_scope(self.workspace_root, request.path)
        path_filter = path_scope.relative if path_scope is not None else None
        backend_language = resolve_backend_language(request.language, request.framework)
        backend_type = self._resolve_backend_type(request.kind)

        internal = self.adapter.list_endpoints(
            workspace_root=self.workspace_root,
            options={"language": backend_language, "type": backend_type},
        )

        normalized = self._normalize_items(internal.by_language)
        filtered = self._filter_items(
            items=normalized,
            language=request.language,
            framework=request.framework,
            kind=request.kind,
            path_filter=path_filter,
        )

        total_count = len(filtered)
        truncated = total_count > request.limit
        items = filtered[: request.limit]
        status = ResponseStatus.PARTIAL if truncated else ResponseStatus.OK

        errors: list[ErrorItem] = []
        if truncated:
            errors.append(
                ErrorItem(
                    code=ErrorCode.RESULT_TRUNCATED,
                    message=f"Result set exceeded the requested limit of {request.limit} items.",
                    retryable=False,
                    suggestion="Increase limit or narrow the path/language/framework/kind filters.",
                    details={
                        "returned": len(items),
                        "total": total_count,
                        "limit": request.limit,
                    },
                )
            )

        if internal.errors:
            errors.append(
                ErrorItem(
                    code=ErrorCode.BACKEND_EXECUTION_FAILED,
                    message="One or more backend analyzers failed while collecting endpoint data.",
                    retryable=True,
                    suggestion="Retry with narrower filters or inspect backend analyzer errors in meta.backend.errors.",
                    details={"count": len(internal.errors)},
                )
            )
            if status == ResponseStatus.OK:
                status = ResponseStatus.PARTIAL

        return ResponseEnvelope[ListEndpointsData](
            tool=TOOL_NAME,
            status=status,
            summary=self._build_summary(
                count=total_count,
                path=request.path,
                kind=request.kind,
                truncated=truncated,
            ),
            data=ListEndpointsData(
                totalCount=total_count,
                returnedCount=len(items),
                counts=self._build_grouped_counts(filtered),
                items=items,
            ),
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
    ) -> ResponseEnvelope[ListEndpointsData]:
        details = []
        for item in error.errors():
            details.append(
                {
                    "field": ".".join(str(part) for part in item.get("loc", [])),
                    "message": item.get("msg", "Invalid value."),
                    "type": item.get("type", "validation_error"),
                }
            )

        return ResponseEnvelope[ListEndpointsData](
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
        request: ListEndpointsInput,
        error: BackendToolError,
    ) -> ResponseEnvelope[ListEndpointsData]:
        return ResponseEnvelope[ListEndpointsData](
            tool=TOOL_NAME,
            status=ResponseStatus.ERROR,
            summary="Endpoint discovery failed.",
            data=self._empty_data(),
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
    ) -> ResponseEnvelope[ListEndpointsData]:
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

        return ResponseEnvelope[ListEndpointsData](
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

    def _resolve_backend_type(self, kind: PublicEndpointKind) -> str:
        if kind == PublicEndpointKind.ROUTE:
            return "routes"
        return kind.value

    def _normalize_items(
        self, by_language: list[dict[str, Any]]
    ) -> list[PublicEndpointItem]:
        items: dict[tuple[Any, ...], PublicEndpointItem] = {}
        for group in by_language:
            language = self._normalize_language(group.get("language"))
            framework = self._normalize_framework(group.get("framework"))
            for endpoint in group.get("endpoints", []):
                if not isinstance(endpoint, dict):
                    continue
                file_path = self._normalize_relative_path(
                    str(endpoint.get("file", "")).strip()
                )
                line = int(endpoint.get("line", 1))
                name = str(endpoint.get("name", "")).strip() or "<unknown>"
                kind = self._normalize_kind(endpoint.get("kind"), endpoint.get("group"))
                endpoint_path = self._normalize_optional_text(endpoint.get("path"))
                key = (file_path, line, name, kind.value, endpoint_path or "")
                items[key] = PublicEndpointItem(
                    name=name,
                    kind=kind,
                    path=endpoint_path,
                    file=file_path,
                    line=line,
                    language=language or infer_language_from_path(file_path),
                    framework=framework,
                )

        return sorted(
            items.values(),
            key=lambda item: (
                item.kind.value,
                item.path or "",
                item.file,
                item.line,
                item.name,
            ),
        )

    def _filter_items(
        self,
        *,
        items: list[PublicEndpointItem],
        language: PublicLanguage | None,
        framework: PublicFramework | None,
        kind: PublicEndpointKind,
        path_filter: Path | None,
    ) -> list[PublicEndpointItem]:
        filtered = items

        if language is not None:
            filtered = [item for item in filtered if item.language == language]

        if framework is not None:
            filtered = [item for item in filtered if item.framework == framework]

        if kind != PublicEndpointKind.ANY:
            filtered = [item for item in filtered if item.kind == kind]

        if path_filter is not None:
            prefix = path_filter.as_posix()
            filtered = [
                item
                for item in filtered
                if item.file == prefix or item.file.startswith(f"{prefix}/")
            ]

        return filtered

    def _build_grouped_counts(
        self, items: list[PublicEndpointItem]
    ) -> ListEndpointsGroupedCounts:
        by_kind: dict[str, int] = {}
        by_language: dict[str, int] = {}
        by_framework: dict[str, int] = {}

        for item in items:
            by_kind[item.kind.value] = by_kind.get(item.kind.value, 0) + 1
            if item.language is not None:
                by_language[item.language.value] = (
                    by_language.get(item.language.value, 0) + 1
                )
            if item.framework is not None:
                by_framework[item.framework.value] = (
                    by_framework.get(item.framework.value, 0) + 1
                )

        return ListEndpointsGroupedCounts(
            byKind=dict(sorted(by_kind.items())),
            byLanguage=dict(sorted(by_language.items())),
            byFramework=dict(sorted(by_framework.items())),
        )

    def _normalize_kind(
        self, kind_value: Any, group_value: Any | None = None
    ) -> PublicEndpointKind:
        for candidate in (
            self._normalize_kind_token(kind_value),
            self._normalize_kind_token(group_value),
        ):
            if candidate in GRAPHQL_KIND_TOKENS:
                return PublicEndpointKind.GRAPHQL
            if candidate in REST_KIND_TOKENS:
                return PublicEndpointKind.REST
            if candidate in ROUTE_KIND_TOKENS:
                return PublicEndpointKind.ROUTE
        return PublicEndpointKind.ANY

    def _normalize_kind_token(self, value: Any) -> str:
        normalized = str(value or "").strip().lower()
        if not normalized:
            return ""
        normalized = re.sub(r"[^a-z0-9]+", "_", normalized).strip("_")
        return normalized

    def _normalize_language(self, value: Any) -> PublicLanguage | None:
        normalized = str(value or "").strip().lower()
        if normalized == "java":
            return PublicLanguage.JAVA
        if normalized == "typescript":
            return PublicLanguage.TYPESCRIPT
        if normalized == "javascript":
            return PublicLanguage.JAVASCRIPT
        return None

    def _normalize_framework(self, value: Any) -> PublicFramework | None:
        normalized = str(value or "").strip().lower()
        if normalized in {"react-router", "react-router-7"}:
            return PublicFramework.REACT_ROUTER
        if normalized == "spring":
            return PublicFramework.SPRING
        return None

    def _normalize_relative_path(self, path_value: str) -> str:
        candidate = Path(path_value)
        if candidate.is_absolute():
            try:
                return candidate.relative_to(self.workspace_root).as_posix()
            except ValueError:
                return candidate.as_posix()
        return candidate.as_posix()

    def _normalize_optional_text(self, value: Any) -> str | None:
        if value is None:
            return None
        normalized = str(value).strip()
        return normalized or None

    def _build_summary(
        self,
        *,
        count: int,
        path: str | None,
        kind: PublicEndpointKind,
        truncated: bool,
    ) -> str:
        subject = self._summary_subject(kind=kind, count=count)
        if path:
            subject = f"{subject} under '{path}'"
        if count == 0:
            return f"No {subject} found."
        if truncated:
            return f"Found {count} {subject} and returned a truncated subset."
        return f"Found {count} {subject}."

    def _empty_data(self) -> ListEndpointsData:
        return ListEndpointsData(
            totalCount=0,
            returnedCount=0,
            counts=ListEndpointsGroupedCounts(),
            items=[],
        )

    def _summary_subject(self, *, kind: PublicEndpointKind, count: int) -> str:
        if kind == PublicEndpointKind.ANY:
            return "endpoint or route" if count == 1 else "endpoints and routes"
        if kind == PublicEndpointKind.GRAPHQL:
            return "GraphQL endpoint" if count == 1 else "GraphQL endpoints"
        if kind == PublicEndpointKind.REST:
            return "REST endpoint" if count == 1 else "REST endpoints"
        return "frontend route" if count == 1 else "frontend routes"
