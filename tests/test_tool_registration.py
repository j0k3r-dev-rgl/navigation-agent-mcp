from __future__ import annotations

from navigation_mcp.app import create_mcp

from helpers import run_async


def test_v1_tools_are_registered_with_expected_discoverable_schema(tmp_path):
    mcp = create_mcp(workspace_root=tmp_path)

    tools = run_async(mcp.list_tools())
    tools_by_name = {tool.name: tool for tool in tools}

    assert set(tools_by_name) == {
        "code.find_symbol",
        "code.search_text",
        "code.trace_symbol",
        "code.trace_callers",
        "code.list_endpoints",
        "code.inspect_tree",
    }

    inspect_tree_schema = tools_by_name["code.inspect_tree"].inputSchema
    assert inspect_tree_schema["properties"]["max_depth"]["default"] == 3
    assert inspect_tree_schema["properties"]["max_depth"]["maximum"] == 20

    find_symbol_schema = tools_by_name["code.find_symbol"].inputSchema
    assert find_symbol_schema["required"] == ["symbol"]
    assert "MatchMode" in find_symbol_schema["$defs"]

    trace_symbol_schema = tools_by_name["code.trace_symbol"].inputSchema
    assert set(trace_symbol_schema["required"]) == {"path", "symbol"}

    trace_callers_schema = tools_by_name["code.trace_callers"].inputSchema
    max_depth_schema = trace_callers_schema["properties"]["max_depth"]["anyOf"][0]
    assert max_depth_schema["minimum"] == 1
    assert max_depth_schema["maximum"] == 8

    list_endpoints_schema = tools_by_name["code.list_endpoints"].inputSchema
    assert list_endpoints_schema["properties"]["limit"]["default"] == 50
    assert "PublicEndpointKind" in list_endpoints_schema["$defs"]
