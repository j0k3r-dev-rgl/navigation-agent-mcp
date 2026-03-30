from __future__ import annotations

import os
from pathlib import Path

from mcp.server.fastmcp import FastMCP

from navigation_mcp.adapters.internal_tools import (
    InternalFindSymbolAdapter,
    InternalInspectTreeAdapter,
    InternalListEndpointsAdapter,
    InternalSearchTextAdapter,
    InternalTraceCallersAdapter,
    InternalTraceSymbolAdapter,
)
from navigation_mcp.services import (
    FindSymbolService,
    InspectTreeService,
    ListEndpointsService,
    SearchTextService,
    TraceCallersService,
    TraceSymbolService,
)
from navigation_mcp.tools import register_code_tools


def create_mcp(
    *,
    workspace_root: Path | None = None,
    find_symbol_script: Path | None = None,
    list_endpoints_script: Path | None = None,
    trace_callers_script: Path | None = None,
    trace_symbol_script: Path | None = None,
    host: str = "127.0.0.1",
    port: int = 8000,
    streamable_http_path: str = "/mcp",
) -> FastMCP:
    resolved_workspace = (workspace_root or Path.cwd()).resolve()
    find_symbol_override = find_symbol_script
    if find_symbol_override is None:
        configured = os.getenv("NAVIGATION_MCP_FIND_SYMBOL_SCRIPT")
        find_symbol_override = Path(configured).expanduser() if configured else None

    list_endpoints_override = list_endpoints_script
    if list_endpoints_override is None:
        configured = os.getenv("NAVIGATION_MCP_LIST_ENDPOINTS_SCRIPT")
        list_endpoints_override = Path(configured).expanduser() if configured else None

    trace_symbol_override = trace_symbol_script
    if trace_symbol_override is None:
        configured = os.getenv("NAVIGATION_MCP_TRACE_SYMBOL_SCRIPT")
        trace_symbol_override = Path(configured).expanduser() if configured else None

    trace_callers_override = trace_callers_script
    if trace_callers_override is None:
        configured = os.getenv("NAVIGATION_MCP_TRACE_CALLERS_SCRIPT")
        trace_callers_override = Path(configured).expanduser() if configured else None

    mcp = FastMCP(
        "navigation-agent-mcp",
        host=host,
        port=port,
        streamable_http_path=streamable_http_path,
    )
    find_symbol_service = FindSymbolService(
        workspace_root=resolved_workspace,
        adapter=InternalFindSymbolAdapter(find_symbol_override),
    )
    inspect_tree_service = InspectTreeService(
        workspace_root=resolved_workspace,
        adapter=InternalInspectTreeAdapter(),
    )
    list_endpoints_service = ListEndpointsService(
        workspace_root=resolved_workspace,
        adapter=InternalListEndpointsAdapter(list_endpoints_override),
    )
    search_text_service = SearchTextService(
        workspace_root=resolved_workspace,
        adapter=InternalSearchTextAdapter(),
    )
    trace_symbol_service = TraceSymbolService(
        workspace_root=resolved_workspace,
        adapter=InternalTraceSymbolAdapter(trace_symbol_override),
    )
    trace_callers_service = TraceCallersService(
        workspace_root=resolved_workspace,
        adapter=InternalTraceCallersAdapter(trace_callers_override),
    )
    register_code_tools(
        mcp=mcp,
        find_symbol_service=find_symbol_service,
        inspect_tree_service=inspect_tree_service,
        list_endpoints_service=list_endpoints_service,
        search_text_service=search_text_service,
        trace_callers_service=trace_callers_service,
        trace_symbol_service=trace_symbol_service,
    )
    return mcp
