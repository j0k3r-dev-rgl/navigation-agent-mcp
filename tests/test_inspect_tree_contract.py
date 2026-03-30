from __future__ import annotations

from navigation_mcp.adapters.internal_tools.inspect_tree_adapter import MAX_TREE_ITEMS
from navigation_mcp.app import create_mcp

from helpers import run_async, unwrap_tool_result


def _call_inspect_tree(mcp, **arguments):
    return unwrap_tool_result(run_async(mcp.call_tool("code.inspect_tree", arguments)))


def test_inspect_tree_include_hidden_still_respects_hard_ignore_list(tmp_path):
    (tmp_path / "src").mkdir()
    (tmp_path / "src" / "main.py").write_text("print('ok')\n")
    (tmp_path / ".hidden").mkdir()
    (tmp_path / ".hidden" / "note.txt").write_text("secret\n")
    (tmp_path / ".git").mkdir()
    (tmp_path / ".git" / "config").write_text("[core]\n")
    (tmp_path / "node_modules").mkdir()
    (tmp_path / "node_modules" / "pkg.js").write_text("module.exports = {}\n")

    mcp = create_mcp(workspace_root=tmp_path)
    response = _call_inspect_tree(mcp, max_depth=2, include_hidden=True)

    assert response["status"] == "ok"
    paths = {item["path"] for item in response["data"]["items"]}

    assert ".hidden" in paths
    assert ".hidden/note.txt" in paths
    assert "src" in paths
    assert "src/main.py" in paths
    assert not any(path == ".git" or path.startswith(".git/") for path in paths)
    assert not any(
        path == "node_modules" or path.startswith("node_modules/") for path in paths
    )


def test_inspect_tree_scoped_to_hard_ignored_directory_returns_empty_result(tmp_path):
    (tmp_path / ".git").mkdir()
    (tmp_path / ".git" / "config").write_text("[core]\n")

    mcp = create_mcp(workspace_root=tmp_path)
    response = _call_inspect_tree(mcp, path=".git", max_depth=3, include_hidden=True)

    assert response["status"] == "ok"
    assert response["data"]["root"] == ".git"
    assert response["data"]["entryCount"] == 0
    assert response["meta"]["resolvedPath"] == ".git"
    assert response["meta"]["truncated"] is False


def test_inspect_tree_reports_truncation_at_safety_cap(tmp_path):
    many = tmp_path / "many"
    many.mkdir()
    for index in range(MAX_TREE_ITEMS + 1):
        (many / f"file_{index:04d}.txt").write_text("x\n")

    mcp = create_mcp(workspace_root=tmp_path)
    response = _call_inspect_tree(mcp, path="many", max_depth=1)

    assert response["status"] == "partial"
    assert response["data"]["entryCount"] == MAX_TREE_ITEMS
    assert response["meta"]["truncated"] is True
    assert response["meta"]["counts"]["returnedCount"] == MAX_TREE_ITEMS
    assert response["meta"]["counts"]["totalMatched"] is None
    assert response["errors"][0]["code"] == "RESULT_TRUNCATED"


def test_inspect_tree_returns_file_not_found_for_missing_scope(tmp_path):
    mcp = create_mcp(workspace_root=tmp_path)
    response = _call_inspect_tree(mcp, path="missing")

    assert response["status"] == "error"
    assert response["summary"] == "Path not found."
    assert response["errors"][0]["code"] == "FILE_NOT_FOUND"
    assert response["errors"][0]["details"] == {"path": "missing"}
