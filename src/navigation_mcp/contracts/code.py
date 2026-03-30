from __future__ import annotations

from enum import Enum
import re

from pydantic import BaseModel, Field, field_validator


class PublicLanguage(str, Enum):
    TYPESCRIPT = "typescript"
    JAVASCRIPT = "javascript"
    JAVA = "java"


class PublicFramework(str, Enum):
    REACT_ROUTER = "react-router"
    SPRING = "spring"


class PublicEndpointKind(str, Enum):
    ANY = "any"
    GRAPHQL = "graphql"
    REST = "rest"
    ROUTE = "route"


class PublicSymbolKind(str, Enum):
    ANY = "any"
    CLASS = "class"
    INTERFACE = "interface"
    FUNCTION = "function"
    METHOD = "method"
    TYPE = "type"
    ENUM = "enum"
    CONSTRUCTOR = "constructor"
    ANNOTATION = "annotation"


class MatchMode(str, Enum):
    EXACT = "exact"
    FUZZY = "fuzzy"


class FindSymbolInput(BaseModel):
    symbol: str = Field(min_length=1, description="Symbol name to locate.")
    language: PublicLanguage | None = Field(
        default=None,
        description="Optional language filter. If omitted, framework may infer the effective language.",
    )
    framework: PublicFramework | None = Field(
        default=None,
        description="Optional framework hint used for language inference and result filtering.",
    )
    kind: PublicSymbolKind = Field(
        default=PublicSymbolKind.ANY,
        description="Stable public symbol kind filter.",
    )
    match: MatchMode = Field(
        default=MatchMode.EXACT,
        description="Exact matches by default; fuzzy broadens the search.",
    )
    path: str | None = Field(
        default=None,
        description="Optional workspace-relative or absolute path scope.",
    )
    limit: int = Field(
        default=50,
        ge=1,
        le=200,
        description="Maximum number of returned items.",
    )

    @field_validator("symbol")
    @classmethod
    def validate_symbol(cls, value: str) -> str:
        normalized = value.strip()
        if not normalized:
            raise ValueError("symbol must not be empty")
        return normalized

    @field_validator("path")
    @classmethod
    def validate_path(cls, value: str | None) -> str | None:
        if value is None:
            return value
        normalized = value.strip()
        return normalized or None


class ListEndpointsInput(BaseModel):
    path: str | None = Field(
        default=None,
        description="Optional workspace-relative or absolute path scope.",
    )
    language: PublicLanguage | None = Field(
        default=None,
        description="Optional language filter. If omitted, framework may infer the effective language.",
    )
    framework: PublicFramework | None = Field(
        default=None,
        description="Optional framework hint used for language inference and result filtering.",
    )
    kind: PublicEndpointKind = Field(
        default=PublicEndpointKind.ANY,
        description="Stable public endpoint kind filter.",
    )
    limit: int = Field(
        default=50,
        ge=1,
        le=200,
        description="Maximum number of returned items.",
    )

    @field_validator("path")
    @classmethod
    def validate_list_endpoints_path(cls, value: str | None) -> str | None:
        if value is None:
            return value
        normalized = value.strip()
        return normalized or None


class PublicEndpointItem(BaseModel):
    name: str
    kind: PublicEndpointKind
    path: str | None = None
    file: str
    line: int = Field(ge=1)
    language: PublicLanguage | None = None
    framework: PublicFramework | None = None


class ListEndpointsGroupedCounts(BaseModel):
    byKind: dict[str, int] = Field(default_factory=dict)
    byLanguage: dict[str, int] = Field(default_factory=dict)
    byFramework: dict[str, int] = Field(default_factory=dict)


class ListEndpointsData(BaseModel):
    totalCount: int = Field(ge=0)
    returnedCount: int = Field(ge=0)
    counts: ListEndpointsGroupedCounts = Field(
        default_factory=ListEndpointsGroupedCounts
    )
    items: list[PublicEndpointItem] = Field(default_factory=list)


class PublicSymbolDefinition(BaseModel):
    symbol: str
    kind: PublicSymbolKind
    path: str
    line: int = Field(ge=1)
    language: PublicLanguage | None = None


class FindSymbolData(BaseModel):
    count: int = Field(ge=0)
    returnedCount: int = Field(ge=0)
    totalMatched: int = Field(ge=0)
    items: list[PublicSymbolDefinition] = Field(default_factory=list)


class TraceSymbolInput(BaseModel):
    path: str = Field(min_length=1, description="Starting file path for the trace")
    symbol: str = Field(min_length=1, description="Symbol name to trace forward")
    language: PublicLanguage | None = Field(default=None)
    framework: PublicFramework | None = Field(default=None)

    @field_validator("path", "symbol")
    @classmethod
    def validate_required_string(cls, value: str) -> str:
        normalized = value.strip()
        if not normalized:
            raise ValueError("value must not be empty")
        return normalized


class TraceSymbolEntrypoint(BaseModel):
    path: str
    symbol: str
    language: PublicLanguage | None = None


class TraceSymbolFile(BaseModel):
    path: str
    language: PublicLanguage | None = None


class TraceSymbolData(BaseModel):
    entrypoint: TraceSymbolEntrypoint
    fileCount: int = Field(ge=0)
    items: list[TraceSymbolFile] = Field(default_factory=list)


class TraceCallersInput(BaseModel):
    path: str = Field(min_length=1, description="Starting file path for reverse trace")
    symbol: str = Field(
        min_length=1, description="Symbol name to trace incoming callers for"
    )
    language: PublicLanguage | None = Field(default=None)
    framework: PublicFramework | None = Field(default=None)
    recursive: bool = Field(
        default=False,
        description="Enable recursive reverse traversal beyond direct callers.",
    )
    max_depth: int | None = Field(
        default=None,
        ge=1,
        le=8,
        description="Maximum recursive reverse-trace depth. Defaults to 3 when recursive=true.",
    )

    @field_validator("path", "symbol")
    @classmethod
    def validate_trace_callers_required_string(cls, value: str) -> str:
        normalized = value.strip()
        if not normalized:
            raise ValueError("value must not be empty")
        return normalized


class TraceCallersTarget(BaseModel):
    path: str
    symbol: str
    language: PublicLanguage | None = None


class TraceCallerRecord(BaseModel):
    path: str
    line: int = Field(ge=1)
    column: int | None = Field(default=None, ge=1)
    caller: str
    callerSymbol: str | None = None
    relation: str
    language: PublicLanguage | None = None
    snippet: str | None = None
    receiverType: str | None = None


class TraceCallersRecursiveVia(BaseModel):
    relation: str | None = None
    line: int | None = Field(default=None, ge=1)
    column: int | None = Field(default=None, ge=1)
    snippet: str | None = None


class TraceCallersRecursiveNode(BaseModel):
    key: str
    path: str
    symbol: str
    depth: int = Field(ge=0)
    via: TraceCallersRecursiveVia | None = None


class TraceCallersRecursivePathSegment(BaseModel):
    path: str
    symbol: str
    depth: int = Field(ge=0)


class TraceCallersRecursiveCycle(BaseModel):
    fromKey: str
    toKey: str
    path: list[str] = Field(default_factory=list)


class TraceCallersRecursiveTruncatedNode(BaseModel):
    key: str
    path: str
    symbol: str
    depth: int = Field(ge=0)


class TraceCallersProbableEntryPoint(BaseModel):
    key: str | None = None
    path: str
    symbol: str
    depth: int | None = Field(default=None, ge=0)
    reasons: list[str] = Field(default_factory=list)
    probable: bool | None = None
    pathFromTarget: list[TraceCallersRecursivePathSegment] = Field(default_factory=list)


class TraceCallersCallsTarget(BaseModel):
    path: str
    symbol: str


class TraceCallersClassificationRecord(BaseModel):
    path: str
    symbol: str
    caller: str
    depth: int = Field(ge=0)
    line: int = Field(ge=1)
    column: int | None = Field(default=None, ge=1)
    relation: str
    language: PublicLanguage | None = None
    receiverType: str | None = None
    snippet: str | None = None
    calls: TraceCallersCallsTarget
    pathFromTarget: list[TraceCallersRecursivePathSegment] = Field(default_factory=list)


class TraceCallersImplementationInterface(BaseModel):
    name: str | None = None
    path: str | None = None
    symbol: str | None = None


class TraceCallersImplementationReference(BaseModel):
    path: str
    symbol: str | None = None


class TraceCallersImplementationInterfaceChain(BaseModel):
    kind: str
    probable: bool | None = None
    interface: TraceCallersImplementationInterface | None = None
    implementation: TraceCallersImplementationReference | None = None
    implementations: list[TraceCallersImplementationReference] = Field(
        default_factory=list
    )
    callers: list[TraceCallersClassificationRecord] = Field(default_factory=list)


class TraceCallersRecursiveSummary(BaseModel):
    directCallerCount: int = Field(ge=0)
    indirectCallerCount: int = Field(ge=0)
    probablePublicEntryPointCount: int = Field(ge=0)
    implementationInterfaceChainCount: int = Field(ge=0)


class TraceCallersRecursiveClassifications(BaseModel):
    summary: TraceCallersRecursiveSummary
    directCallers: list[TraceCallersClassificationRecord] = Field(default_factory=list)
    indirectCallers: list[TraceCallersClassificationRecord] = Field(
        default_factory=list
    )
    probablePublicEntryPoints: list[TraceCallersProbableEntryPoint] = Field(
        default_factory=list
    )
    implementationInterfaceChain: list[TraceCallersImplementationInterfaceChain] = (
        Field(default_factory=list)
    )


class TraceCallersRecursiveData(BaseModel):
    enabled: bool = True
    root: TraceCallersRecursiveNode
    maxDepth: int = Field(ge=1)
    maxDepthObserved: int = Field(ge=0)
    nodeCount: int = Field(ge=0)
    edgeCount: int = Field(ge=0)
    pathCount: int = Field(ge=0)
    nodes: list[TraceCallersRecursiveNode] = Field(default_factory=list)
    adjacency: dict[str, list[str]] = Field(default_factory=dict)
    paths: list[list[TraceCallersRecursivePathSegment]] = Field(default_factory=list)
    cycles: list[TraceCallersRecursiveCycle] = Field(default_factory=list)
    truncated: list[TraceCallersRecursiveTruncatedNode] = Field(default_factory=list)
    probableEntryPoints: list[TraceCallersProbableEntryPoint] = Field(
        default_factory=list
    )
    classifications: TraceCallersRecursiveClassifications


class TraceCallersData(BaseModel):
    target: TraceCallersTarget
    count: int = Field(ge=0)
    returnedCount: int = Field(ge=0)
    items: list[TraceCallerRecord] = Field(default_factory=list)
    recursive: TraceCallersRecursiveData | None = None


class SearchTextInput(BaseModel):
    query: str = Field(min_length=1, description="Text or regex pattern to search")
    path: str | None = Field(
        default=None,
        description="Optional workspace-relative or absolute path scope.",
    )
    language: PublicLanguage | None = Field(
        default=None,
        description="Optional language filter. If omitted, framework may infer the effective language.",
    )
    framework: PublicFramework | None = Field(
        default=None,
        description="Optional framework hint used for language inference.",
    )
    include: str | None = Field(
        default=None,
        description="Optional extra glob include filter such as '*.tsx' or 'src/**'.",
    )
    regex: bool = Field(
        default=False,
        description="Interpret query as a regular expression when true.",
    )
    context: int = Field(
        default=1,
        ge=0,
        le=10,
        description="Number of context lines before and after each match.",
    )
    limit: int = Field(
        default=50,
        ge=1,
        le=200,
        description="Maximum number of matched files to return.",
    )

    @field_validator("query")
    @classmethod
    def validate_query(cls, value: str) -> str:
        normalized = value.strip()
        if not normalized:
            raise ValueError("query must not be empty")
        return normalized

    @field_validator("path", "include")
    @classmethod
    def validate_optional_string(cls, value: str | None) -> str | None:
        if value is None:
            return value
        normalized = value.strip()
        return normalized or None


class SearchTextContextLine(BaseModel):
    line: int = Field(ge=1)
    text: str


class SearchTextSubmatch(BaseModel):
    start: int = Field(ge=0)
    end: int = Field(ge=0)
    text: str


class SearchTextMatch(BaseModel):
    line: int = Field(ge=1)
    text: str
    submatches: list[SearchTextSubmatch] = Field(default_factory=list)
    before: list[SearchTextContextLine] = Field(default_factory=list)
    after: list[SearchTextContextLine] = Field(default_factory=list)


class SearchTextFileMatch(BaseModel):
    path: str
    language: PublicLanguage | None = None
    matchCount: int = Field(ge=0)
    matches: list[SearchTextMatch] = Field(default_factory=list)


class SearchTextData(BaseModel):
    fileCount: int = Field(ge=0)
    matchCount: int = Field(ge=0)
    totalFileCount: int = Field(ge=0)
    totalMatchCount: int = Field(ge=0)
    items: list[SearchTextFileMatch] = Field(default_factory=list)


class InspectTreeInput(BaseModel):
    path: str | None = Field(
        default=None,
        description="Optional workspace-relative or absolute file or directory scope.",
    )
    max_depth: int = Field(
        default=3,
        ge=0,
        le=20,
        description="Maximum depth relative to the resolved scope root.",
    )
    extensions: list[str] = Field(
        default_factory=list,
        description="Optional file extension filter. Directories remain visible.",
    )
    file_pattern: str | None = Field(
        default=None,
        description="Optional filename glob such as '*.py'.",
    )
    include_stats: bool = Field(
        default=False,
        description="Include size, modified time, and symlink metadata.",
    )
    include_hidden: bool = Field(
        default=False,
        description="Include hidden files except for the hard ignore list.",
    )

    @field_validator("path", "file_pattern")
    @classmethod
    def validate_optional_tree_string(cls, value: str | None) -> str | None:
        if value is None:
            return value
        normalized = value.strip()
        return normalized or None

    @field_validator("extensions", mode="before")
    @classmethod
    def validate_extensions(cls, value: object) -> list[str]:
        if value is None:
            return []
        if not isinstance(value, list):
            raise ValueError("extensions must be a list of strings")

        normalized_extensions: list[str] = []
        for item in value:
            if not isinstance(item, str):
                raise ValueError("extensions must contain only strings")
            normalized = item.strip().lower()
            if not normalized:
                continue
            if normalized == ".":
                raise ValueError("extensions entries must not be '.'")
            if not normalized.startswith("."):
                normalized = f".{normalized}"
            normalized_extensions.append(normalized)

        return sorted(set(normalized_extensions))


class InspectTreeItemStats(BaseModel):
    sizeBytes: int = Field(ge=0)
    modifiedAt: str
    isSymlink: bool = False


class InspectTreeItem(BaseModel):
    path: str
    name: str
    type: str
    depth: int = Field(ge=1)
    extension: str | None = None
    stats: InspectTreeItemStats | None = None


class InspectTreeData(BaseModel):
    root: str
    entryCount: int = Field(ge=0)
    items: list[InspectTreeItem] = Field(default_factory=list)


_PUBLIC_SYMBOL_KIND_ALIASES: dict[str, PublicSymbolKind] = {
    "annotation": PublicSymbolKind.ANNOTATION,
    "annotation_type": PublicSymbolKind.ANNOTATION,
    "annotationtype": PublicSymbolKind.ANNOTATION,
    "class": PublicSymbolKind.CLASS,
    "class_declaration": PublicSymbolKind.CLASS,
    "classdeclaration": PublicSymbolKind.CLASS,
    "constructor": PublicSymbolKind.CONSTRUCTOR,
    "constructor_declaration": PublicSymbolKind.CONSTRUCTOR,
    "constructordeclaration": PublicSymbolKind.CONSTRUCTOR,
    "enum": PublicSymbolKind.ENUM,
    "enum_declaration": PublicSymbolKind.ENUM,
    "enumdeclaration": PublicSymbolKind.ENUM,
    "function": PublicSymbolKind.FUNCTION,
    "function_declaration": PublicSymbolKind.FUNCTION,
    "functiondeclaration": PublicSymbolKind.FUNCTION,
    "function_signature": PublicSymbolKind.FUNCTION,
    "functionsignature": PublicSymbolKind.FUNCTION,
    "interface": PublicSymbolKind.INTERFACE,
    "interface_declaration": PublicSymbolKind.INTERFACE,
    "interfacedeclaration": PublicSymbolKind.INTERFACE,
    "method": PublicSymbolKind.METHOD,
    "method_declaration": PublicSymbolKind.METHOD,
    "methoddeclaration": PublicSymbolKind.METHOD,
    "record": PublicSymbolKind.TYPE,
    "record_declaration": PublicSymbolKind.TYPE,
    "recorddeclaration": PublicSymbolKind.TYPE,
    "type": PublicSymbolKind.TYPE,
    "type_alias": PublicSymbolKind.TYPE,
    "typealias": PublicSymbolKind.TYPE,
    "type_declaration": PublicSymbolKind.TYPE,
    "typedefinition": PublicSymbolKind.TYPE,
}


def normalize_public_symbol_kind(value: object) -> PublicSymbolKind:
    normalized = re.sub(r"[^a-z0-9]+", "_", str(value or "").strip().lower()).strip("_")
    return _PUBLIC_SYMBOL_KIND_ALIASES.get(normalized, PublicSymbolKind.ANY)
