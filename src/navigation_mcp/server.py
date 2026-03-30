from __future__ import annotations

import argparse
import os
from pathlib import Path

from navigation_mcp.app import create_mcp


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Run the navigation-agent-mcp server.")
    parser.add_argument(
        "--transport",
        choices=("stdio", "streamable-http"),
        default="stdio",
        help="MCP transport to expose.",
    )
    parser.add_argument(
        "--host", default="127.0.0.1", help="Host for streamable HTTP transport."
    )
    parser.add_argument(
        "--port", type=int, default=8000, help="Port for streamable HTTP transport."
    )
    parser.add_argument(
        "--path", default="/mcp", help="HTTP path for streamable HTTP transport."
    )
    parser.add_argument(
        "--workspace-root",
        default=os.getenv("NAVIGATION_MCP_WORKSPACE_ROOT", str(Path.cwd())),
        help="Workspace root to analyze. Defaults to the current working directory.",
    )
    parser.add_argument(
        "--find-symbol-script",
        default=os.getenv("NAVIGATION_MCP_FIND_SYMBOL_SCRIPT"),
        help="Optional override for the internal find_symbol adapter script.",
    )
    parser.add_argument(
        "--list-endpoints-script",
        default=os.getenv("NAVIGATION_MCP_LIST_ENDPOINTS_SCRIPT"),
        help="Optional override for the internal list_endpoints adapter script.",
    )
    parser.add_argument(
        "--trace-callers-script",
        default=os.getenv("NAVIGATION_MCP_TRACE_CALLERS_SCRIPT"),
        help="Optional override for the internal trace_callers adapter script.",
    )
    parser.add_argument(
        "--trace-symbol-script",
        default=os.getenv("NAVIGATION_MCP_TRACE_SYMBOL_SCRIPT"),
        help="Optional override for the internal trace_symbol adapter script.",
    )
    return parser


def main() -> None:
    args = build_parser().parse_args()

    server = create_mcp(
        workspace_root=Path(args.workspace_root).expanduser(),
        find_symbol_script=Path(args.find_symbol_script).expanduser()
        if args.find_symbol_script
        else None,
        list_endpoints_script=Path(args.list_endpoints_script).expanduser()
        if args.list_endpoints_script
        else None,
        trace_callers_script=Path(args.trace_callers_script).expanduser()
        if args.trace_callers_script
        else None,
        trace_symbol_script=Path(args.trace_symbol_script).expanduser()
        if args.trace_symbol_script
        else None,
        host=args.host,
        port=args.port,
        streamable_http_path=args.path,
    )

    if args.transport == "stdio":
        server.run(transport="stdio")
        return

    server.run(transport="streamable-http")
