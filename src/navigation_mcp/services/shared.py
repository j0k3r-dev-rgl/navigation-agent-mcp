from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from navigation_mcp.contracts.code import (
    PublicFramework,
    PublicLanguage,
    TraceCallersRecursiveData,
)
from navigation_mcp.contracts.common import ErrorCode, ResponseMeta

DEFAULT_TRACE_CALLERS_MAX_DEPTH = 3
MAX_TRACE_CALLERS_MAX_DEPTH = 8
MAX_RECURSIVE_NODES = 200
MAX_RECURSIVE_PATHS = 50
MAX_RECURSIVE_CYCLES = 50
MAX_RECURSIVE_TRUNCATED_NODES = 100
MAX_RECURSIVE_ENTRY_POINTS = 50
MAX_RECURSIVE_CLASSIFICATION_ITEMS = 100
MAX_RECURSIVE_INTERFACE_CHAINS = 50


class PathResolutionError(Exception):
    def __init__(self, *, code: ErrorCode, path_value: str) -> None:
        super().__init__(path_value)
        self.code = code
        self.path_value = path_value


@dataclass(frozen=True, slots=True)
class ResolvedScope:
    absolute: Path
    relative: Path | None

    @property
    def public_path(self) -> str:
        if self.relative is None:
            return "."
        return self.relative.as_posix()


def resolve_optional_scope(
    workspace_root: Path,
    path_value: str | None,
    *,
    must_exist: bool = True,
) -> ResolvedScope | None:
    if not path_value:
        return None
    return resolve_required_scope(
        workspace_root,
        path_value,
        must_exist=must_exist,
    )


def resolve_required_scope(
    workspace_root: Path,
    path_value: str,
    *,
    must_exist: bool = True,
) -> ResolvedScope:
    candidate = Path(path_value)
    resolved = (
        candidate.resolve()
        if candidate.is_absolute()
        else (workspace_root / candidate).resolve()
    )

    if resolved != workspace_root and workspace_root not in resolved.parents:
        raise PathResolutionError(
            code=ErrorCode.PATH_OUTSIDE_WORKSPACE,
            path_value=path_value,
        )
    if must_exist and not resolved.exists():
        raise PathResolutionError(code=ErrorCode.FILE_NOT_FOUND, path_value=path_value)

    relative = resolved.relative_to(workspace_root)
    return ResolvedScope(
        absolute=resolved,
        relative=None if relative == Path(".") else relative,
    )


def resolve_effective_language(
    language: PublicLanguage | None,
    framework: PublicFramework | None,
) -> PublicLanguage | None:
    if language is not None:
        return language
    if framework == PublicFramework.REACT_ROUTER:
        return PublicLanguage.TYPESCRIPT
    if framework == PublicFramework.SPRING:
        return PublicLanguage.JAVA
    return None


def resolve_backend_language(
    language: PublicLanguage | None,
    framework: PublicFramework | None,
) -> str:
    effective = resolve_effective_language(language, framework)
    if effective == PublicLanguage.JAVA:
        return "java"
    if effective == PublicLanguage.PYTHON:
        return "python"
    if effective in {PublicLanguage.TYPESCRIPT, PublicLanguage.JAVASCRIPT}:
        return "typescript"
    return "auto"


def infer_language_from_path(path_value: str) -> PublicLanguage | None:
    suffix = Path(path_value).suffix.lower()
    if suffix in {".ts", ".tsx"}:
        return PublicLanguage.TYPESCRIPT
    if suffix in {".js", ".jsx"}:
        return PublicLanguage.JAVASCRIPT
    if suffix == ".java":
        return PublicLanguage.JAVA
    if suffix == ".py":
        return PublicLanguage.PYTHON
    return None


def build_response_meta(
    *,
    query: dict[str, object],
    resolved_path: str | None = None,
    truncated: bool = False,
    counts: dict[str, int | None] | None = None,
    detection: dict[str, str | None] | None = None,
) -> ResponseMeta:
    return ResponseMeta(
        query=query,
        resolvedPath=resolved_path,
        truncated=truncated,
        counts=counts or {},
        detection=detection or {},
    )


def prune_recursive_trace_payload(
    data: TraceCallersRecursiveData,
) -> tuple[TraceCallersRecursiveData, bool]:
    truncated = False

    nodes = data.nodes[:MAX_RECURSIVE_NODES]
    if len(nodes) < len(data.nodes):
        truncated = True
    allowed_keys = {data.root.key, *(node.key for node in nodes)}

    adjacency = {
        key: [child for child in children if child in allowed_keys]
        for key, children in data.adjacency.items()
        if key in allowed_keys
    }
    if len(adjacency) < len(data.adjacency):
        truncated = True

    paths = data.paths[:MAX_RECURSIVE_PATHS]
    cycles = data.cycles[:MAX_RECURSIVE_CYCLES]
    truncated_nodes = data.truncated[:MAX_RECURSIVE_TRUNCATED_NODES]
    probable_entry_points = data.probableEntryPoints[:MAX_RECURSIVE_ENTRY_POINTS]
    direct_callers = data.classifications.directCallers[
        :MAX_RECURSIVE_CLASSIFICATION_ITEMS
    ]
    indirect_callers = data.classifications.indirectCallers[
        :MAX_RECURSIVE_CLASSIFICATION_ITEMS
    ]
    probable_public_entry_points = data.classifications.probablePublicEntryPoints[
        :MAX_RECURSIVE_ENTRY_POINTS
    ]
    implementation_chains = data.classifications.implementationInterfaceChain[
        :MAX_RECURSIVE_INTERFACE_CHAINS
    ]

    if len(paths) < len(data.paths):
        truncated = True
    if len(cycles) < len(data.cycles):
        truncated = True
    if len(truncated_nodes) < len(data.truncated):
        truncated = True
    if len(probable_entry_points) < len(data.probableEntryPoints):
        truncated = True
    if len(direct_callers) < len(data.classifications.directCallers):
        truncated = True
    if len(indirect_callers) < len(data.classifications.indirectCallers):
        truncated = True
    if len(probable_public_entry_points) < len(
        data.classifications.probablePublicEntryPoints
    ):
        truncated = True
    if len(implementation_chains) < len(
        data.classifications.implementationInterfaceChain
    ):
        truncated = True

    return (
        data.model_copy(
            update={
                "nodes": nodes,
                "adjacency": adjacency,
                "paths": paths,
                "cycles": cycles,
                "truncated": truncated_nodes,
                "probableEntryPoints": probable_entry_points,
                "classifications": data.classifications.model_copy(
                    update={
                        "directCallers": direct_callers,
                        "indirectCallers": indirect_callers,
                        "probablePublicEntryPoints": probable_public_entry_points,
                        "implementationInterfaceChain": implementation_chains,
                    }
                ),
            }
        ),
        truncated,
    )
