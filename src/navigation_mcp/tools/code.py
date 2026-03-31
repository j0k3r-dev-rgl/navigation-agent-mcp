from __future__ import annotations

from typing import Annotated, Any

from pydantic import Field, ValidationError

from navigation_mcp.adapters.internal_tools import BackendToolError
from navigation_mcp.contracts.code import (
    FindSymbolInput,
    FindSymbolData,
    InspectTreeInput,
    InspectTreeData,
    ListEndpointsInput,
    ListEndpointsData,
    MatchMode,
    PublicEndpointKind,
    PublicFramework,
    PublicLanguage,
    PublicSymbolKind,
    SearchTextInput,
    SearchTextData,
    TraceCallersInput,
    TraceCallersData,
    TraceSymbolInput,
    TraceSymbolData,
)
from navigation_mcp.contracts.common import ErrorCode, ResponseEnvelope
from navigation_mcp.services import (
    FindSymbolService,
    InspectTreeService,
    ListEndpointsService,
    SearchTextService,
    TraceCallersService,
    TraceSymbolService,
)
from navigation_mcp.services.shared import PathResolutionError


def register_code_tools(
    *,
    mcp: Any,
    find_symbol_service: FindSymbolService,
    inspect_tree_service: InspectTreeService,
    list_endpoints_service: ListEndpointsService,
    search_text_service: SearchTextService,
    trace_callers_service: TraceCallersService,
    trace_symbol_service: TraceSymbolService,
) -> None:
    @mcp.tool(
        name="code.inspect_tree",
        title="Inspect workspace tree",
        description="Inspect the workspace file tree without reading file contents. Supports path scoping, depth limits, extension filters, filename globs, and optional stats.",
        structured_output=True,
    )
    def code_inspect_tree(
        path: Annotated[
            str | None,
            Field(
                description="Optional workspace-relative or absolute file/directory scope.",
            ),
        ] = None,
        max_depth: Annotated[
            int,
            Field(
                description="Maximum depth relative to the resolved scope root.",
                ge=0,
                le=20,
            ),
        ] = 3,
        extensions: Annotated[
            list[str] | None,
            Field(
                description="Optional file extension filter such as ['.py', '.ts']. Directories remain visible.",
            ),
        ] = None,
        file_pattern: Annotated[
            str | None,
            Field(description="Optional filename glob such as '*.py'."),
        ] = None,
        include_stats: Annotated[
            bool,
            Field(description="Include size, modified time, and symlink metadata."),
        ] = False,
        include_hidden: Annotated[
            bool,
            Field(description="Include hidden entries except the hard ignore list."),
        ] = False,
    ) -> ResponseEnvelope[InspectTreeData]:
        payload = {
            "path": path,
            "max_depth": max_depth,
            "extensions": extensions or [],
            "file_pattern": file_pattern,
            "include_stats": include_stats,
            "include_hidden": include_hidden,
        }

        try:
            request = InspectTreeInput.model_validate(payload)
        except ValidationError as error:
            return inspect_tree_service.error_from_validation(error)

        try:
            return inspect_tree_service.execute(request)
        except PathResolutionError as error:
            return inspect_tree_service.error_from_path(
                request_payload=request.model_dump(mode="json"),
                path_value=error.path_value,
                exists=error.code is ErrorCode.PATH_OUTSIDE_WORKSPACE,
            )

    @mcp.tool(
        name="code.list_endpoints",
        title="List endpoints and routes",
        description="List backend endpoints and frontend routes in the workspace. Supports path scoping plus language, framework, kind, and limit filters.",
        structured_output=True,
    )
    def code_list_endpoints(
        path: Annotated[
            str | None,
            Field(
                description="Optional workspace-relative or absolute file/directory scope."
            ),
        ] = None,
        language: Annotated[
            PublicLanguage | None,
            Field(
                description="Optional language filter: typescript, javascript, java, or python."
            ),
        ] = None,
        framework: Annotated[
            PublicFramework | None,
            Field(description="Optional framework hint: react-router or spring."),
        ] = None,
        kind: Annotated[
            PublicEndpointKind,
            Field(
                description="Stable endpoint kind filter: any, graphql, rest, or route."
            ),
        ] = PublicEndpointKind.ANY,
        limit: Annotated[
            int,
            Field(description="Maximum number of returned items.", ge=1, le=200),
        ] = 50,
    ) -> ResponseEnvelope[ListEndpointsData]:
        payload = {
            "path": path,
            "language": language,
            "framework": framework,
            "kind": kind,
            "limit": limit,
        }

        try:
            request = ListEndpointsInput.model_validate(payload)
        except ValidationError as error:
            return list_endpoints_service.error_from_validation(error)

        try:
            return list_endpoints_service.execute(request)
        except PathResolutionError as error:
            return list_endpoints_service.error_from_path(
                request_payload=request.model_dump(mode="json"),
                path_value=error.path_value,
                exists=error.code is ErrorCode.PATH_OUTSIDE_WORKSPACE,
            )
        except BackendToolError as error:
            return list_endpoints_service.error_from_backend(
                request=request, error=error
            )

    @mcp.tool(
        name="code.find_symbol",
        title="Find symbol definitions",
        description="Locate symbol definitions in the workspace by name. Supports exact or fuzzy matching, path scoping, and language/framework/kind filtering.",
        structured_output=True,
    )
    def code_find_symbol(
        symbol: Annotated[
            str,
            Field(description="Symbol name to locate."),
        ],
        language: Annotated[
            PublicLanguage | None,
            Field(
                description="Optional language filter: typescript, javascript, java, or python."
            ),
        ] = None,
        framework: Annotated[
            PublicFramework | None,
            Field(description="Optional framework hint: react-router or spring."),
        ] = None,
        kind: Annotated[
            PublicSymbolKind,
            Field(description="Stable symbol kind filter."),
        ] = PublicSymbolKind.ANY,
        match: Annotated[
            MatchMode,
            Field(description="Match mode: exact or fuzzy."),
        ] = MatchMode.EXACT,
        path: Annotated[
            str | None,
            Field(description="Optional workspace-relative or absolute path scope."),
        ] = None,
        limit: Annotated[
            int,
            Field(description="Maximum number of returned items.", ge=1, le=200),
        ] = 50,
    ) -> ResponseEnvelope[FindSymbolData]:
        payload = {
            "symbol": symbol,
            "language": language,
            "framework": framework,
            "kind": kind,
            "match": match,
            "path": path,
            "limit": limit,
        }

        try:
            request = FindSymbolInput.model_validate(payload)
        except ValidationError as error:
            return find_symbol_service.error_from_validation(error)

        try:
            return find_symbol_service.execute(request)
        except PathResolutionError as error:
            return find_symbol_service.error_from_path(
                request_payload=request.model_dump(mode="json"),
                path_value=error.path_value,
                exists=error.code is ErrorCode.PATH_OUTSIDE_WORKSPACE,
            )
        except BackendToolError as error:
            return find_symbol_service.error_from_backend(request=request, error=error)

    @mcp.tool(
        name="code.search_text",
        title="Search text",
        description="Search text or regex patterns across the workspace with file, language, path, and context controls.",
        structured_output=True,
    )
    def code_search_text(
        query: Annotated[
            str, Field(description="Text or regex pattern to search for.")
        ],
        path: Annotated[
            str | None,
            Field(description="Optional workspace-relative or absolute path scope."),
        ] = None,
        language: Annotated[
            PublicLanguage | None,
            Field(
                description="Optional language filter: typescript, javascript, or java."
            ),
        ] = None,
        framework: Annotated[
            PublicFramework | None,
            Field(description="Optional framework hint: react-router or spring."),
        ] = None,
        include: Annotated[
            str | None,
            Field(
                description="Optional additional include glob such as '*.tsx' or 'src/**'."
            ),
        ] = None,
        regex: Annotated[
            bool,
            Field(description="Interpret query as a regular expression when true."),
        ] = False,
        context: Annotated[
            int,
            Field(
                description="Number of context lines before and after each match.",
                ge=0,
                le=10,
            ),
        ] = 1,
        limit: Annotated[
            int,
            Field(
                description="Maximum number of matched files to return.", ge=1, le=200
            ),
        ] = 50,
    ) -> ResponseEnvelope[SearchTextData]:
        payload = {
            "query": query,
            "path": path,
            "language": language,
            "framework": framework,
            "include": include,
            "regex": regex,
            "context": context,
            "limit": limit,
        }

        try:
            request = SearchTextInput.model_validate(payload)
        except ValidationError as error:
            return search_text_service.error_from_validation(error)

        try:
            return search_text_service.execute(request)
        except PathResolutionError as error:
            return search_text_service.error_from_path(
                request_payload=request.model_dump(mode="json"),
                path_value=error.path_value,
                exists=error.code is ErrorCode.PATH_OUTSIDE_WORKSPACE,
            )
        except BackendToolError as error:
            return search_text_service.error_from_backend(request=request, error=error)

    @mcp.tool(
        name="code.trace_symbol",
        title="Trace symbol forward",
        description="Trace a symbol forward from a starting file to related workspace files. The starting path must exist inside the workspace.",
        structured_output=True,
    )
    def code_trace_symbol(
        path: Annotated[
            str,
            Field(description="Workspace-relative or absolute starting file path."),
        ],
        symbol: Annotated[str, Field(description="Symbol name to trace forward.")],
        language: Annotated[
            PublicLanguage | None,
            Field(
                description="Optional language hint: typescript, javascript, or java."
            ),
        ] = None,
        framework: Annotated[
            PublicFramework | None,
            Field(description="Optional framework hint: react-router or spring."),
        ] = None,
    ) -> ResponseEnvelope[TraceSymbolData]:
        payload = {
            "path": path,
            "symbol": symbol,
            "language": language,
            "framework": framework,
        }

        try:
            request = TraceSymbolInput.model_validate(payload)
        except ValidationError as error:
            return trace_symbol_service.error_from_validation(error)

        try:
            return trace_symbol_service.execute(request)
        except PathResolutionError as error:
            return trace_symbol_service.error_from_path(
                request_payload=request.model_dump(mode="json"),
                path_value=error.path_value,
                exists=error.code is ErrorCode.PATH_OUTSIDE_WORKSPACE,
            )
        except BackendToolError as error:
            return trace_symbol_service.error_from_backend(request=request, error=error)

    @mcp.tool(
        name="code.trace_callers",
        title="Trace incoming callers",
        description="Trace incoming callers for a symbol from a starting file. Recursive mode supports reverse traversal up to a bounded max_depth and may return a partial response for safety.",
        structured_output=True,
    )
    def code_trace_callers(
        path: Annotated[
            str,
            Field(description="Workspace-relative or absolute starting file path."),
        ],
        symbol: Annotated[
            str,
            Field(description="Symbol name to trace incoming callers for."),
        ],
        language: Annotated[
            PublicLanguage | None,
            Field(
                description="Optional language hint: typescript, javascript, or java."
            ),
        ] = None,
        framework: Annotated[
            PublicFramework | None,
            Field(description="Optional framework hint: react-router or spring."),
        ] = None,
        recursive: Annotated[
            bool,
            Field(
                description="Enable recursive reverse traversal beyond direct callers."
            ),
        ] = False,
        max_depth: Annotated[
            int | None,
            Field(
                description="Maximum recursive reverse-trace depth. Allowed range: 1-8. Defaults to 3 when recursive=true.",
                ge=1,
                le=8,
            ),
        ] = None,
    ) -> ResponseEnvelope[TraceCallersData]:
        payload = {
            "path": path,
            "symbol": symbol,
            "language": language,
            "framework": framework,
            "recursive": recursive,
            "max_depth": max_depth,
        }

        try:
            request = TraceCallersInput.model_validate(payload)
        except ValidationError as error:
            return trace_callers_service.error_from_validation(error)

        try:
            return trace_callers_service.execute(request)
        except PathResolutionError as error:
            return trace_callers_service.error_from_path(
                request_payload=request.model_dump(mode="json"),
                path_value=error.path_value,
                exists=error.code is ErrorCode.PATH_OUTSIDE_WORKSPACE,
            )
        except BackendToolError as error:
            return trace_callers_service.error_from_backend(
                request=request, error=error
            )
