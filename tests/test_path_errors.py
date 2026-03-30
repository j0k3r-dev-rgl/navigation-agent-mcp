from __future__ import annotations

from navigation_mcp.app import create_mcp

from helpers import run_async, unwrap_tool_result


def test_search_text_rejects_paths_outside_workspace(tmp_path):
    mcp = create_mcp(workspace_root=tmp_path)

    response = unwrap_tool_result(
        run_async(
            mcp.call_tool(
                "code.search_text",
                {"query": "loader", "path": "../outside"},
            )
        )
    )

    assert response["status"] == "error"
    assert response["summary"] == "Path validation failed."
    assert response["errors"][0]["code"] == "PATH_OUTSIDE_WORKSPACE"
    assert response["errors"][0]["details"] == {"path": "../outside"}
