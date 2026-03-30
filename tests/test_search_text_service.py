from __future__ import annotations

from navigation_mcp.adapters.internal_tools.search_text_adapter import (
    InternalSearchTextResult,
)
from navigation_mcp.contracts.code import PublicFramework, SearchTextInput
from navigation_mcp.contracts.common import ResponseStatus
from navigation_mcp.services.search_text_service import SearchTextService


class StubSearchTextAdapter:
    def __init__(self, result: InternalSearchTextResult) -> None:
        self.result = result
        self.calls: list[dict[str, object]] = []

    def search_text(self, **kwargs):
        self.calls.append(kwargs)
        return self.result


def test_search_text_service_normalizes_truncated_response(tmp_path):
    (tmp_path / "src").mkdir()

    adapter = StubSearchTextAdapter(
        InternalSearchTextResult(
            files=[
                {
                    "path": "src/a.tsx",
                    "matchCount": 2,
                    "matches": [
                        {
                            "line": 3,
                            "text": "useLoaderData()",
                            "submatches": [
                                {"start": 0, "end": 13, "text": "useLoaderData"}
                            ],
                            "before": [{"line": 2, "text": "const value = true;"}],
                            "after": [{"line": 4, "text": "return value;"}],
                        },
                        {
                            "line": 10,
                            "text": "loader()",
                            "submatches": [{"start": 0, "end": 6, "text": "loader"}],
                            "before": [],
                            "after": [],
                        },
                    ],
                },
                {
                    "path": "src/b.ts",
                    "matchCount": 1,
                    "matches": [
                        {
                            "line": 1,
                            "text": "loader",
                            "submatches": [{"start": 0, "end": 6, "text": "loader"}],
                            "before": [],
                            "after": [],
                        }
                    ],
                },
            ],
            file_count=2,
            match_count=3,
            raw_events=5,
        )
    )
    service = SearchTextService(workspace_root=tmp_path, adapter=adapter)

    response = service.execute(
        SearchTextInput(
            query="loader",
            path="src",
            framework=PublicFramework.REACT_ROUTER,
            limit=1,
        )
    )

    assert response.status is ResponseStatus.PARTIAL
    assert response.summary == (
        "Found 3 text matches across 2 files for 'loader' and returned a truncated subset."
    )
    assert response.data.fileCount == 1
    assert response.data.matchCount == 2
    assert response.data.totalFileCount == 2
    assert response.data.totalMatchCount == 3
    assert response.data.items[0].path == "src/a.tsx"
    assert response.data.items[0].language.value == "typescript"
    assert response.errors[0].code == "RESULT_TRUNCATED"
    assert response.meta.resolvedPath == "src"
    assert response.meta.truncated is True
    assert response.meta.detection["effectiveLanguage"] == "typescript"
    assert response.meta.detection["framework"] == "react-router"
    assert response.meta.counts == {
        "returnedFileCount": 1,
        "totalFileCount": 2,
        "returnedMatchCount": 2,
        "totalMatchCount": 3,
    }
    assert adapter.calls[0]["path_filter"].as_posix() == "src"
    assert adapter.calls[0]["language_globs"] == ["*.ts", "*.tsx"]


def test_search_text_service_error_from_backend_preserves_request_meta(tmp_path):
    service = SearchTextService(
        workspace_root=tmp_path, adapter=StubSearchTextAdapter(None)
    )
    request = SearchTextInput(query="loader", path="src")

    from navigation_mcp.adapters.internal_tools import BackendToolError

    response = service.error_from_backend(
        request=request,
        error=BackendToolError(
            code="BACKEND_EXECUTION_FAILED",
            message="ripgrep failed",
            retryable=True,
            suggestion="Retry later.",
            details={"returncode": 2},
        ),
    )

    assert response.status is ResponseStatus.ERROR
    assert response.summary == "Text search failed."
    assert response.errors[0].message == "ripgrep failed"
    assert response.errors[0].retryable is True
    assert response.meta.query == request.model_dump(mode="json")
