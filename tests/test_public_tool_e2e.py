from __future__ import annotations

from pathlib import Path
import textwrap

from navigation_mcp.app import create_mcp

from helpers import run_async, unwrap_tool_result


def _call_tool(mcp, name: str, **arguments):
    return unwrap_tool_result(run_async(mcp.call_tool(name, arguments)))


def _write_stub_script(path: Path, body: str) -> Path:
    path.write_text(textwrap.dedent(body))
    return path


def _create_workspace(tmp_path: Path) -> Path:
    (tmp_path / "front" / "app" / "routes" / "api").mkdir(parents=True)
    (tmp_path / "back" / "src" / "main" / "java" / "com" / "acme").mkdir(parents=True)

    (tmp_path / "front" / "app" / "routes" / "dashboard.tsx").write_text(
        "export async function loader() {}\n"
    )
    (tmp_path / "front" / "app" / "routes" / "layout.tsx").write_text(
        "export default function Layout() { return null }\n"
    )
    (tmp_path / "front" / "app" / "routes" / "api" / "session.ts").write_text(
        "export async function action() {}\n"
    )
    (
        tmp_path
        / "back"
        / "src"
        / "main"
        / "java"
        / "com"
        / "acme"
        / "HomeController.java"
    ).write_text("class HomeController {}\n")
    (
        tmp_path
        / "back"
        / "src"
        / "main"
        / "java"
        / "com"
        / "acme"
        / "NavGraphQLController.java"
    ).write_text("class NavGraphQLController {}\n")
    (
        tmp_path
        / "back"
        / "src"
        / "main"
        / "java"
        / "com"
        / "acme"
        / "NavRestController.java"
    ).write_text("class NavRestController {}\n")
    return tmp_path


def _create_script_overrides(tmp_path: Path) -> dict[str, Path]:
    scripts_dir = tmp_path / "stubs"
    scripts_dir.mkdir()

    find_symbol_script = _write_stub_script(
        scripts_dir / "find_symbol_stub.py",
        """
        import json
        import sys
        from pathlib import Path

        workspace_root = Path(sys.argv[1])
        symbol = sys.argv[2]
        options = json.loads(sys.argv[3])

        print(json.dumps({
            "matches": [
                {
                    "name": symbol,
                    "kind": "function_declaration",
                    "file": str(workspace_root / "front/app/routes/dashboard.tsx"),
                    "line": 5,
                },
                {
                    "name": "Layout",
                    "kind": "function",
                    "file": "front/app/routes/layout.tsx",
                    "line": 1,
                },
                {
                    "name": "HomeController",
                    "kind": "class_declaration",
                    "file": str(workspace_root / "back/src/main/java/com/acme/HomeController.java"),
                    "line": 7,
                },
            ],
            "count": 3,
            "options": options,
        }))
        """,
    )

    trace_symbol_script = _write_stub_script(
        scripts_dir / "trace_symbol_stub.py",
        """
        import json
        import sys
        from pathlib import Path

        workspace_root = Path(sys.argv[1])
        path = Path(sys.argv[2])
        symbol = sys.argv[3]
        options = json.loads(sys.argv[4])

        print(json.dumps({
            "files": [
                str(workspace_root / "front/app/routes/dashboard.tsx"),
                "back/src/main/java/com/acme/HomeController.java",
                str(path),
            ],
            "count": 3,
            "options": options,
            "symbol": symbol,
        }))
        """,
    )

    trace_callers_script = _write_stub_script(
        scripts_dir / "trace_callers_stub.py",
        """
        import json
        import sys
        from pathlib import Path

        workspace_root = Path(sys.argv[1])
        path = Path(sys.argv[2])
        symbol = sys.argv[3]
        options = json.loads(sys.argv[4])

        payload = {
            "target": {
                "file": str(path),
                "symbol": symbol,
            },
            "callers": [
                {
                    "file": str(workspace_root / "front/app/routes/layout.tsx"),
                    "line": 9,
                    "column": 3,
                    "relation": "calls",
                    "caller": "Layout",
                    "traverse_symbol": "loader",
                    "snippet": "loader()",
                },
                {
                    "file": "back/src/main/java/com/acme/HomeController.java",
                    "line": 22,
                    "relation": "references",
                    "caller": "HomeController#index",
                    "receiverType": "HomeController",
                },
            ],
            "count": 2,
            "options": options,
            "mode": "recursive" if options.get("recursive") else "direct",
            "directSummary": {"count": 2},
        }

        if options.get("recursive"):
            payload["recursiveResult"] = {
                "enabled": True,
                "root": {
                    "key": "target",
                    "file": str(path),
                    "symbol": symbol,
                    "depth": 0,
                },
                "maxDepth": options.get("maxDepth", 3),
                "maxDepthObserved": 2,
                "nodeCount": 3,
                "edgeCount": 2,
                "pathCount": 1,
                "nodes": [
                    {
                        "key": "target",
                        "file": str(path),
                        "symbol": symbol,
                        "depth": 0,
                    },
                    {
                        "key": "layout",
                        "file": str(workspace_root / "front/app/routes/layout.tsx"),
                        "symbol": "Layout",
                        "depth": 1,
                        "via": {
                            "relation": "calls",
                            "line": 9,
                            "column": 3,
                            "snippet": "loader()",
                        },
                    },
                    {
                        "key": "controller",
                        "file": str(workspace_root / "back/src/main/java/com/acme/HomeController.java"),
                        "symbol": "HomeController#index",
                        "depth": 2,
                    },
                ],
                "adjacency": {
                    "target": ["layout"],
                    "layout": ["controller"],
                },
                "paths": [[
                    {"file": str(path), "symbol": symbol, "depth": 0},
                    {
                        "file": str(workspace_root / "front/app/routes/layout.tsx"),
                        "symbol": "Layout",
                        "depth": 1,
                    },
                    {
                        "file": str(workspace_root / "back/src/main/java/com/acme/HomeController.java"),
                        "symbol": "HomeController#index",
                        "depth": 2,
                    },
                ]],
                "cycles": [],
                "truncated": [],
                "probableEntryPoints": [
                    {
                        "key": "controller",
                        "file": str(workspace_root / "back/src/main/java/com/acme/HomeController.java"),
                        "symbol": "HomeController#index",
                        "depth": 2,
                        "reasons": ["public controller method"],
                        "probable": True,
                        "pathFromTarget": [
                            {"file": str(path), "symbol": symbol, "depth": 0},
                            {
                                "file": str(workspace_root / "back/src/main/java/com/acme/HomeController.java"),
                                "symbol": "HomeController#index",
                                "depth": 2,
                            },
                        ],
                    }
                ],
                "classifications": {
                    "summary": {
                        "directCallerCount": 1,
                        "indirectCallerCount": 1,
                        "probablePublicEntryPointCount": 1,
                        "implementationInterfaceChainCount": 1,
                    },
                    "directCallers": [
                        {
                            "file": str(workspace_root / "front/app/routes/layout.tsx"),
                            "symbol": "Layout",
                            "caller": "Layout",
                            "depth": 1,
                            "line": 9,
                            "column": 3,
                            "relation": "calls",
                            "calls": {"file": str(path), "symbol": symbol},
                            "pathFromTarget": [
                                {"file": str(path), "symbol": symbol, "depth": 0},
                                {
                                    "file": str(workspace_root / "front/app/routes/layout.tsx"),
                                    "symbol": "Layout",
                                    "depth": 1,
                                },
                            ],
                        }
                    ],
                    "indirectCallers": [
                        {
                            "file": str(workspace_root / "back/src/main/java/com/acme/HomeController.java"),
                            "symbol": "HomeController#index",
                            "caller": "HomeController#index",
                            "depth": 2,
                            "line": 22,
                            "relation": "references",
                            "calls": {
                                "file": str(workspace_root / "front/app/routes/layout.tsx"),
                                "symbol": "Layout",
                            },
                        }
                    ],
                    "probablePublicEntryPoints": [
                        {
                            "key": "controller",
                            "file": str(workspace_root / "back/src/main/java/com/acme/HomeController.java"),
                            "symbol": "HomeController#index",
                            "depth": 2,
                            "reasons": ["public controller method"],
                            "probable": True,
                        }
                    ],
                    "implementationInterfaceChain": [
                        {
                            "kind": "implementation",
                            "probable": True,
                            "interface": {
                                "name": "NavigationLoader",
                                "file": str(workspace_root / "front/app/routes/layout.tsx"),
                                "symbol": "NavigationLoader",
                            },
                            "implementation": {
                                "file": str(workspace_root / "back/src/main/java/com/acme/HomeController.java"),
                                "symbol": "HomeController#index",
                            },
                            "implementations": [
                                {
                                    "file": str(workspace_root / "back/src/main/java/com/acme/HomeController.java"),
                                    "symbol": "HomeController#index",
                                }
                            ],
                            "callers": [],
                        }
                    ],
                },
            }

        print(json.dumps(payload))
        """,
    )

    list_endpoints_script = _write_stub_script(
        scripts_dir / "list_endpoints_stub.py",
        """
        import json
        import sys
        from pathlib import Path

        workspace_root = Path(sys.argv[1])
        options = json.loads(sys.argv[2])

        print(json.dumps({
            "byLanguage": [
                {
                    "language": "java",
                    "framework": "spring",
                    "endpoints": [
                        {
                            "name": "getNavigation",
                            "kind": "query",
                            "path": "getNavigation",
                            "file": str(workspace_root / "back/src/main/java/com/acme/NavGraphQLController.java"),
                            "line": 12,
                        },
                        {
                            "name": "createNavigation",
                            "group": "requests",
                            "path": "/api/navigation",
                            "file": str(workspace_root / "back/src/main/java/com/acme/NavRestController.java"),
                            "line": 20,
                        },
                    ],
                },
                {
                    "language": "typescript",
                    "framework": "react-router-7",
                    "endpoints": [
                        {
                            "name": "loader",
                            "kind": "loader",
                            "path": "/dashboard",
                            "file": str(workspace_root / "front/app/routes/dashboard.tsx"),
                            "line": 5,
                        },
                        {
                            "name": "sessionResource",
                            "group": "resources",
                            "path": "/api/session",
                            "file": "front/app/routes/api/session.ts",
                            "line": 3,
                        },
                    ],
                },
            ],
            "errors": [],
            "options": options,
        }))
        """,
    )

    return {
        "find_symbol_script": find_symbol_script,
        "trace_symbol_script": trace_symbol_script,
        "trace_callers_script": trace_callers_script,
        "list_endpoints_script": list_endpoints_script,
    }


def _create_test_mcp(tmp_path):
    workspace = _create_workspace(tmp_path)
    overrides = _create_script_overrides(tmp_path)
    return create_mcp(workspace_root=workspace, **overrides)


def test_find_symbol_normalizes_public_results_and_filters_by_scope_and_language(
    tmp_path,
):
    mcp = _create_test_mcp(tmp_path)

    response = _call_tool(
        mcp,
        "code.find_symbol",
        symbol="loader",
        language="typescript",
        path="front/app/routes",
    )

    assert response["status"] == "ok"
    assert response["summary"] == "Found 2 symbol definitions for 'loader'."
    assert response["data"]["count"] == 2
    assert response["data"]["returnedCount"] == 2
    assert [item["path"] for item in response["data"]["items"]] == [
        "front/app/routes/dashboard.tsx",
        "front/app/routes/layout.tsx",
    ]
    assert [item["kind"] for item in response["data"]["items"]] == [
        "function",
        "function",
    ]
    assert all(item["language"] == "typescript" for item in response["data"]["items"])
    assert response["meta"]["resolvedPath"] == "front/app/routes"
    assert response["meta"]["counts"] == {"returnedCount": 2, "totalMatched": 2}


def test_find_symbol_returns_file_not_found_for_missing_scope(tmp_path):
    mcp = _create_test_mcp(tmp_path)

    response = _call_tool(mcp, "code.find_symbol", symbol="loader", path="missing")

    assert response["status"] == "error"
    assert response["summary"] == "Path not found."
    assert response["errors"][0]["code"] == "FILE_NOT_FOUND"
    assert response["errors"][0]["details"] == {"path": "missing"}


def test_trace_symbol_normalizes_trace_files_from_public_tool_layer(tmp_path):
    mcp = _create_test_mcp(tmp_path)

    response = _call_tool(
        mcp,
        "code.trace_symbol",
        path="front/app/routes/dashboard.tsx",
        symbol="loader",
    )

    assert response["status"] == "ok"
    assert response["data"]["entrypoint"] == {
        "path": "front/app/routes/dashboard.tsx",
        "symbol": "loader",
        "language": "typescript",
    }
    assert response["data"]["fileCount"] == 2
    assert response["data"]["items"] == [
        {"path": "back/src/main/java/com/acme/HomeController.java", "language": "java"},
        {"path": "front/app/routes/dashboard.tsx", "language": "typescript"},
    ]
    assert response["meta"]["resolvedPath"] == "front/app/routes/dashboard.tsx"


def test_trace_callers_returns_direct_mode_results_from_registered_tool(tmp_path):
    mcp = _create_test_mcp(tmp_path)

    response = _call_tool(
        mcp,
        "code.trace_callers",
        path="front/app/routes/dashboard.tsx",
        symbol="loader",
    )

    assert response["status"] == "ok"
    assert response["summary"] == (
        "Found 2 incoming callers for 'loader' from 'front/app/routes/dashboard.tsx'."
    )
    assert response["data"]["target"] == {
        "path": "front/app/routes/dashboard.tsx",
        "symbol": "loader",
        "language": "typescript",
    }
    assert response["data"]["count"] == 2
    assert response["data"]["recursive"] is None
    assert response["data"]["items"][0] == {
        "path": "back/src/main/java/com/acme/HomeController.java",
        "line": 22,
        "column": None,
        "caller": "HomeController#index",
        "callerSymbol": None,
        "relation": "references",
        "language": "java",
        "snippet": None,
        "receiverType": "HomeController",
    }
    assert response["data"]["items"][1]["path"] == "front/app/routes/layout.tsx"
    assert response["data"]["items"][1]["callerSymbol"] == "loader"


def test_trace_callers_returns_recursive_payload_when_requested(tmp_path):
    mcp = _create_test_mcp(tmp_path)

    response = _call_tool(
        mcp,
        "code.trace_callers",
        path="front/app/routes/dashboard.tsx",
        symbol="loader",
        recursive=True,
        max_depth=4,
    )

    assert response["status"] == "ok"
    assert response["summary"] == (
        "Found 2 incoming callers for 'loader' from 'front/app/routes/dashboard.tsx' with recursive reverse trace."
    )
    recursive = response["data"]["recursive"]
    assert recursive["enabled"] is True
    assert recursive["maxDepth"] == 4
    assert recursive["maxDepthObserved"] == 2
    assert recursive["root"] == {
        "key": "target",
        "path": "front/app/routes/dashboard.tsx",
        "symbol": "loader",
        "depth": 0,
        "via": None,
    }
    assert recursive["probableEntryPoints"][0]["path"] == (
        "back/src/main/java/com/acme/HomeController.java"
    )
    assert recursive["classifications"]["summary"] == {
        "directCallerCount": 1,
        "indirectCallerCount": 1,
        "probablePublicEntryPointCount": 1,
        "implementationInterfaceChainCount": 1,
    }
    assert recursive["classifications"]["directCallers"][0]["calls"] == {
        "path": "front/app/routes/dashboard.tsx",
        "symbol": "loader",
    }
    assert response["meta"]["truncated"] is False


def test_list_endpoints_normalizes_graphql_items_from_public_tool(tmp_path):
    mcp = _create_test_mcp(tmp_path)

    response = _call_tool(mcp, "code.list_endpoints", kind="graphql")

    assert response["status"] == "ok"
    assert response["summary"] == "Found 1 GraphQL endpoint."
    assert response["data"]["totalCount"] == 1
    assert response["data"]["counts"] == {
        "byKind": {"graphql": 1},
        "byLanguage": {"java": 1},
        "byFramework": {"spring": 1},
    }
    assert response["data"]["items"] == [
        {
            "name": "getNavigation",
            "kind": "graphql",
            "path": "getNavigation",
            "file": "back/src/main/java/com/acme/NavGraphQLController.java",
            "line": 12,
            "language": "java",
            "framework": "spring",
        }
    ]


def test_list_endpoints_normalizes_route_items_from_backend_kind_and_group(tmp_path):
    mcp = _create_test_mcp(tmp_path)

    response = _call_tool(
        mcp,
        "code.list_endpoints",
        kind="route",
        path="front/app/routes",
    )

    assert response["status"] == "ok"
    assert response["summary"] == "Found 2 frontend routes under 'front/app/routes'."
    assert response["data"]["totalCount"] == 2
    assert response["data"]["counts"] == {
        "byKind": {"route": 2},
        "byLanguage": {"typescript": 2},
        "byFramework": {"react-router": 2},
    }
    assert [item["kind"] for item in response["data"]["items"]] == ["route", "route"]
    assert [item["file"] for item in response["data"]["items"]] == [
        "front/app/routes/api/session.ts",
        "front/app/routes/dashboard.tsx",
    ]
    assert [item["path"] for item in response["data"]["items"]] == [
        "/api/session",
        "/dashboard",
    ]
