from __future__ import annotations

from pathlib import Path
from typing import Any

from pydantic import ValidationError

from navigation_mcp.adapters.internal_tools import (
    BackendToolError,
    InternalTraceCallersAdapter,
)
from navigation_mcp.contracts.code import (
    PublicFramework,
    PublicLanguage,
    TraceCallerRecord,
    TraceCallersClassificationRecord,
    TraceCallersData,
    TraceCallersImplementationInterface,
    TraceCallersImplementationInterfaceChain,
    TraceCallersImplementationReference,
    TraceCallersInput,
    TraceCallersProbableEntryPoint,
    TraceCallersRecursiveClassifications,
    TraceCallersRecursiveCycle,
    TraceCallersRecursiveData,
    TraceCallersRecursiveNode,
    TraceCallersRecursivePathSegment,
    TraceCallersRecursiveSummary,
    TraceCallersRecursiveTruncatedNode,
    TraceCallersRecursiveVia,
    TraceCallersTarget,
)
from navigation_mcp.contracts.common import (
    ErrorCode,
    ErrorItem,
    ResponseEnvelope,
    ResponseStatus,
)
from navigation_mcp.services.shared import (
    DEFAULT_TRACE_CALLERS_MAX_DEPTH,
    MAX_TRACE_CALLERS_MAX_DEPTH,
    build_response_meta,
    infer_language_from_path,
    prune_recursive_trace_payload,
    resolve_backend_language,
    resolve_effective_language,
    resolve_required_scope,
)

TOOL_NAME = "code.trace_callers"


class TraceCallersService:
    def __init__(
        self, *, workspace_root: Path, adapter: InternalTraceCallersAdapter
    ) -> None:
        self.workspace_root = workspace_root.resolve()
        self.adapter = adapter

    def execute(self, request: TraceCallersInput) -> ResponseEnvelope[TraceCallersData]:
        start_scope = resolve_required_scope(self.workspace_root, request.path)
        start_path = start_scope.relative or Path(".")
        backend_language = resolve_backend_language(request.language, request.framework)
        options = {
            "language": backend_language,
            "recursive": request.recursive,
        }
        if request.recursive:
            options["maxDepth"] = self._resolve_max_depth(request.max_depth)

        internal = self.adapter.trace_callers(
            workspace_root=self.workspace_root,
            path=start_path,
            symbol=request.symbol,
            options=options,
        )

        callers = self._normalize_callers(internal.callers)
        errors: list[ErrorItem] = []
        status = ResponseStatus.OK
        recursive_data = (
            self._normalize_recursive_data(internal.recursive_result)
            if request.recursive and internal.recursive_result
            else None
        )
        recursive_payload_truncated = False
        if recursive_data is not None:
            recursive_data, recursive_payload_truncated = prune_recursive_trace_payload(
                recursive_data
            )
            if recursive_payload_truncated:
                status = ResponseStatus.PARTIAL
                errors.append(
                    ErrorItem(
                        code=ErrorCode.RESULT_TRUNCATED,
                        message="Recursive reverse-trace payload exceeded the V1 response safety caps.",
                        retryable=False,
                        suggestion="Retry with a lower max_depth or disable recursive mode for the direct caller set only.",
                        details={
                            "maxDepth": options.get("maxDepth"),
                            "nodeCount": recursive_data.nodeCount,
                            "pathCount": recursive_data.pathCount,
                        },
                    )
                )

        data = TraceCallersData(
            target=TraceCallersTarget(
                path=start_path.as_posix(),
                symbol=request.symbol,
                language=infer_language_from_path(start_path.as_posix()),
            ),
            count=len(callers),
            returnedCount=len(callers),
            items=callers,
            recursive=recursive_data,
        )

        return ResponseEnvelope[TraceCallersData](
            tool=TOOL_NAME,
            status=status,
            summary=self._build_summary(
                symbol=request.symbol,
                path=start_path.as_posix(),
                count=data.count,
                recursive=request.recursive,
            ),
            data=data,
            errors=errors,
            meta=build_response_meta(
                query=request.model_dump(mode="json"),
                resolved_path=start_scope.public_path,
                truncated=recursive_payload_truncated,
                counts={"returnedCount": data.count, "totalMatched": data.count},
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
    ) -> ResponseEnvelope[TraceCallersData]:
        details = []
        for item in error.errors():
            details.append(
                {
                    "field": ".".join(str(part) for part in item.get("loc", [])),
                    "message": item.get("msg", "Invalid value."),
                    "type": item.get("type", "validation_error"),
                }
            )

        return ResponseEnvelope[TraceCallersData](
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
        request: TraceCallersInput,
        error: BackendToolError,
    ) -> ResponseEnvelope[TraceCallersData]:
        return ResponseEnvelope[TraceCallersData](
            tool=TOOL_NAME,
            status=ResponseStatus.ERROR,
            summary="Caller trace failed.",
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
    ) -> ResponseEnvelope[TraceCallersData]:
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

        return ResponseEnvelope[TraceCallersData](
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

    def _resolve_max_depth(self, max_depth: int | None) -> int:
        resolved = max_depth or DEFAULT_TRACE_CALLERS_MAX_DEPTH
        return min(resolved, MAX_TRACE_CALLERS_MAX_DEPTH)

    def _normalize_callers(
        self, callers: list[dict[str, Any]]
    ) -> list[TraceCallerRecord]:
        unique: dict[tuple[Any, ...], TraceCallerRecord] = {}
        for item in callers:
            path = str(item.get("file", "")).strip()
            line = int(item.get("line", 1))
            column = item.get("column")
            relation = str(item.get("relation", "unknown")).strip() or "unknown"
            caller = str(item.get("caller", "<unknown>")).strip() or "<unknown>"
            caller_symbol = item.get("traverse_symbol")
            caller_symbol_text = (
                str(caller_symbol).strip() if caller_symbol is not None else None
            )
            key = (path, line, int(column or 0), relation, caller, caller_symbol_text)
            unique[key] = TraceCallerRecord(
                path=path,
                line=line,
                column=int(column) if column is not None else None,
                caller=caller,
                callerSymbol=caller_symbol_text,
                relation=relation,
                language=infer_language_from_path(path),
                snippet=self._normalize_optional_text(item.get("snippet")),
                receiverType=self._normalize_optional_text(item.get("receiverType")),
            )

        return sorted(
            unique.values(),
            key=lambda item: (
                item.path,
                item.line,
                item.column or 0,
                item.callerSymbol or "",
                item.relation,
            ),
        )

    def _normalize_recursive_data(
        self, recursive_result: dict[str, Any]
    ) -> TraceCallersRecursiveData:
        root = recursive_result.get("root", {})
        classifications = recursive_result.get("classifications", {})

        return TraceCallersRecursiveData(
            enabled=bool(recursive_result.get("enabled", True)),
            root=self._normalize_recursive_node(root),
            maxDepth=int(recursive_result.get("maxDepth", 1)),
            maxDepthObserved=int(recursive_result.get("maxDepthObserved", 0)),
            nodeCount=int(recursive_result.get("nodeCount", 0)),
            edgeCount=int(recursive_result.get("edgeCount", 0)),
            pathCount=int(recursive_result.get("pathCount", 0)),
            nodes=[
                self._normalize_recursive_node(node)
                for node in recursive_result.get("nodes", [])
                if isinstance(node, dict)
            ],
            adjacency={
                str(key): [str(child) for child in value]
                for key, value in recursive_result.get("adjacency", {}).items()
                if isinstance(value, list)
            },
            paths=[
                [
                    self._normalize_recursive_path_segment(segment)
                    for segment in path
                    if isinstance(segment, dict)
                ]
                for path in recursive_result.get("paths", [])
                if isinstance(path, list)
            ],
            cycles=[
                TraceCallersRecursiveCycle(
                    fromKey=str(item.get("from", "")),
                    toKey=str(item.get("to", "")),
                    path=[str(part) for part in item.get("path", [])],
                )
                for item in recursive_result.get("cycles", [])
                if isinstance(item, dict)
            ],
            truncated=[
                TraceCallersRecursiveTruncatedNode(
                    key=str(item.get("node", "")),
                    path=str(item.get("file", "")).strip(),
                    symbol=str(item.get("symbol", "")).strip(),
                    depth=int(item.get("depth", 0)),
                )
                for item in recursive_result.get("truncated", [])
                if isinstance(item, dict)
            ],
            probableEntryPoints=[
                self._normalize_probable_entry_point(item)
                for item in recursive_result.get("probableEntryPoints", [])
                if isinstance(item, dict)
            ],
            classifications=TraceCallersRecursiveClassifications(
                summary=TraceCallersRecursiveSummary.model_validate(
                    classifications.get("summary", {})
                ),
                directCallers=[
                    self._normalize_classification_record(item)
                    for item in classifications.get("directCallers", [])
                    if isinstance(item, dict)
                ],
                indirectCallers=[
                    self._normalize_classification_record(item)
                    for item in classifications.get("indirectCallers", [])
                    if isinstance(item, dict)
                ],
                probablePublicEntryPoints=[
                    self._normalize_probable_entry_point(item)
                    for item in classifications.get("probablePublicEntryPoints", [])
                    if isinstance(item, dict)
                ],
                implementationInterfaceChain=[
                    self._normalize_interface_chain(item)
                    for item in classifications.get("implementationInterfaceChain", [])
                    if isinstance(item, dict)
                ],
            ),
        )

    def _normalize_recursive_node(
        self, node: dict[str, Any]
    ) -> TraceCallersRecursiveNode:
        via = node.get("via")
        return TraceCallersRecursiveNode(
            key=str(node.get("key", "")),
            path=str(node.get("file", "")).strip(),
            symbol=str(node.get("symbol", "")).strip(),
            depth=int(node.get("depth", 0)),
            via=TraceCallersRecursiveVia(
                relation=self._normalize_optional_text(via.get("relation")),
                line=int(via.get("line")) if via.get("line") is not None else None,
                column=int(via.get("column"))
                if via.get("column") is not None
                else None,
                snippet=self._normalize_optional_text(via.get("snippet")),
            )
            if isinstance(via, dict)
            else None,
        )

    def _normalize_recursive_path_segment(
        self, segment: dict[str, Any]
    ) -> TraceCallersRecursivePathSegment:
        return TraceCallersRecursivePathSegment(
            path=str(segment.get("file", "")).strip(),
            symbol=str(segment.get("symbol", "")).strip(),
            depth=int(segment.get("depth", 0)),
        )

    def _normalize_probable_entry_point(
        self, item: dict[str, Any]
    ) -> TraceCallersProbableEntryPoint:
        return TraceCallersProbableEntryPoint(
            key=self._normalize_optional_text(item.get("key")),
            path=str(item.get("file", "")).strip(),
            symbol=str(item.get("symbol", "")).strip(),
            depth=int(item.get("depth")) if item.get("depth") is not None else None,
            reasons=[str(reason) for reason in item.get("reasons", [])],
            probable=(
                bool(item.get("probable")) if item.get("probable") is not None else None
            ),
            pathFromTarget=[
                self._normalize_recursive_path_segment(segment)
                for segment in item.get("pathFromTarget", [])
                if isinstance(segment, dict)
            ],
        )

    def _normalize_classification_record(
        self, item: dict[str, Any]
    ) -> TraceCallersClassificationRecord:
        path = str(item.get("file", "")).strip()
        calls = item.get("calls", {})
        return TraceCallersClassificationRecord(
            path=path,
            symbol=str(item.get("symbol", "")).strip(),
            caller=str(item.get("caller", "<unknown>")).strip() or "<unknown>",
            depth=int(item.get("depth", 0)),
            line=int(item.get("line", 1)),
            column=int(item.get("column")) if item.get("column") is not None else None,
            relation=str(item.get("relation", "unknown")).strip() or "unknown",
            language=infer_language_from_path(path),
            receiverType=self._normalize_optional_text(item.get("receiverType")),
            snippet=self._normalize_optional_text(item.get("snippet")),
            calls={
                "path": str(calls.get("file", "")).strip(),
                "symbol": str(calls.get("symbol", "")).strip(),
            },
            pathFromTarget=[
                self._normalize_recursive_path_segment(segment)
                for segment in item.get("pathFromTarget", [])
                if isinstance(segment, dict)
            ],
        )

    def _normalize_interface_chain(
        self, item: dict[str, Any]
    ) -> TraceCallersImplementationInterfaceChain:
        interface = item.get("interface")
        implementation = item.get("implementation")
        implementations = item.get("implementations", [])
        callers = item.get("callers", [])
        return TraceCallersImplementationInterfaceChain(
            kind=str(item.get("kind", "unknown")).strip() or "unknown",
            probable=(
                bool(item.get("probable")) if item.get("probable") is not None else None
            ),
            interface=TraceCallersImplementationInterface(
                name=self._normalize_optional_text(interface.get("name")),
                path=self._normalize_optional_text(interface.get("file")),
                symbol=self._normalize_optional_text(interface.get("symbol")),
            )
            if isinstance(interface, dict)
            else None,
            implementation=TraceCallersImplementationReference(
                path=str(implementation.get("file", "")).strip(),
                symbol=self._normalize_optional_text(implementation.get("symbol")),
            )
            if isinstance(implementation, dict)
            else None,
            implementations=[
                TraceCallersImplementationReference(
                    path=str(entry.get("file", "")).strip(),
                    symbol=self._normalize_optional_text(entry.get("symbol")),
                )
                for entry in implementations
                if isinstance(entry, dict)
            ],
            callers=[
                self._normalize_classification_record(caller)
                for caller in callers
                if isinstance(caller, dict)
            ],
        )

    def _normalize_optional_text(self, value: Any) -> str | None:
        if value is None:
            return None
        normalized = str(value).strip()
        return normalized or None

    def _build_summary(
        self, *, symbol: str, path: str, count: int, recursive: bool
    ) -> str:
        if count == 0:
            return f"Trace completed for incoming callers of '{symbol}' from '{path}' with no callers found."
        if recursive:
            if count == 1:
                return f"Found 1 incoming caller for '{symbol}' from '{path}' with recursive reverse trace."
            return f"Found {count} incoming callers for '{symbol}' from '{path}' with recursive reverse trace."
        if count == 1:
            return f"Found 1 incoming caller for '{symbol}' from '{path}'."
        return f"Found {count} incoming callers for '{symbol}' from '{path}'."

    def _empty_data(self, *, path: str = "", symbol: str = "") -> TraceCallersData:
        return TraceCallersData(
            target=TraceCallersTarget(
                path=path,
                symbol=symbol,
                language=infer_language_from_path(path),
            ),
            count=0,
            returnedCount=0,
            items=[],
            recursive=None,
        )
