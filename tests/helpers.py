from __future__ import annotations

import asyncio
from typing import Any


def run_async(awaitable: Any) -> Any:
    return asyncio.run(awaitable)


def unwrap_tool_result(result: Any) -> dict[str, Any]:
    if isinstance(result, tuple) and len(result) == 2 and isinstance(result[1], dict):
        return result[1]
    if isinstance(result, dict):
        return result
    raise AssertionError(f"Unexpected MCP tool result shape: {type(result)!r}")
