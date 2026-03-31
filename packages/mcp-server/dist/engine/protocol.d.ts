import type { MatchMode, PublicFramework, PublicLanguage, PublicEndpointKind, PublicSymbolKind } from "../contracts/public/code.ts";
export declare const ENGINE_CAPABILITIES: readonly ["workspace.inspect_tree", "workspace.find_symbol", "workspace.list_endpoints", "workspace.search_text", "workspace.trace_symbol", "workspace.trace_callers"];
export type EngineCapability = (typeof ENGINE_CAPABILITIES)[number];
export type AnalyzerLanguage = "auto" | "java" | "typescript" | "python" | "rust";
export interface InspectTreeEnginePayload {
    path: string | null;
    maxDepth: number;
    extensions: string[];
    filePattern: string | null;
    includeStats: boolean;
    includeHidden: boolean;
}
export interface EngineRequest<TPayload = unknown> {
    id: string;
    capability: EngineCapability | string;
    workspaceRoot: string;
    payload: TPayload;
}
export interface EngineError {
    code: string;
    message: string;
    retryable: boolean;
    suggestion?: string | null;
    details: Record<string, unknown>;
}
export interface InspectTreeEngineItemStats {
    sizeBytes: number;
    modifiedAt: string;
    isSymlink: boolean;
}
export interface InspectTreeEngineItem {
    path: string;
    name: string;
    type: "directory" | "file";
    depth: number;
    extension?: string | null;
    stats?: InspectTreeEngineItemStats | null;
}
export interface InspectTreeEngineResult {
    root: string;
    items: InspectTreeEngineItem[];
    truncated: boolean;
    maxItems: number;
    ignoredDirectories: string[];
}
export interface FindSymbolEnginePayload {
    symbol: string;
    path: string | null;
    analyzerLanguage: AnalyzerLanguage;
    publicLanguageFilter: PublicLanguage | null;
    kind: PublicSymbolKind;
    matchMode: MatchMode;
    limit: number;
}
export interface FindSymbolEngineItem {
    symbol: string;
    kind: PublicSymbolKind;
    path: string;
    line: number;
    language: PublicLanguage | null;
}
export interface FindSymbolEngineResult {
    resolvedPath: string | null;
    items: FindSymbolEngineItem[];
    totalMatched: number;
    truncated: boolean;
}
export interface ListEndpointsEnginePayload {
    path: string | null;
    analyzerLanguage: AnalyzerLanguage;
    publicLanguageFilter: PublicLanguage | null;
    publicFrameworkFilter: PublicFramework | null;
    kind: PublicEndpointKind;
    limit: number;
}
export interface ListEndpointsEngineItem {
    name: string;
    kind: string;
    path: string | null;
    file: string;
    line: number;
    language: PublicLanguage | null;
    framework: PublicFramework | null;
}
export interface ListEndpointsEngineCounts {
    byKind: Record<string, number>;
    byLanguage: Record<string, number>;
    byFramework: Record<string, number>;
}
export interface ListEndpointsEngineResult {
    resolvedPath: string | null;
    items: ListEndpointsEngineItem[];
    totalMatched: number;
    truncated: boolean;
    counts: ListEndpointsEngineCounts;
}
export interface SearchTextEnginePayload {
    query: string;
    path: string | null;
    publicLanguageFilter: PublicLanguage | null;
    include: string | null;
    regex: boolean;
    context: number;
    limit: number;
}
export interface SearchTextEngineContextLine {
    line: number;
    text: string;
}
export interface SearchTextEngineSubmatch {
    start: number;
    end: number;
    text: string;
}
export interface SearchTextEngineMatch {
    line: number;
    text: string;
    submatches: SearchTextEngineSubmatch[];
    before: SearchTextEngineContextLine[];
    after: SearchTextEngineContextLine[];
}
export interface SearchTextEngineFileMatch {
    path: string;
    language: PublicLanguage | null;
    matchCount: number;
    matches: SearchTextEngineMatch[];
}
export interface SearchTextEngineResult {
    resolvedPath: string | null;
    items: SearchTextEngineFileMatch[];
    totalFileCount: number;
    totalMatchCount: number;
    truncated: boolean;
}
export interface TraceSymbolEnginePayload {
    path: string;
    symbol: string;
    analyzerLanguage: AnalyzerLanguage;
    publicLanguageFilter: PublicLanguage | null;
}
export interface TraceSymbolEngineItem {
    path: string;
    language: PublicLanguage | null;
}
export interface TraceSymbolEngineResult {
    resolvedPath: string | null;
    items: TraceSymbolEngineItem[];
    totalMatched: number;
    truncated: boolean;
}
export interface TraceCallersEnginePayload {
    path: string;
    symbol: string;
    analyzerLanguage: AnalyzerLanguage;
    publicLanguageFilter: PublicLanguage | null;
    recursive: boolean;
    maxDepth: number | null;
}
export interface TraceCallersEngineTarget {
    path: string;
    symbol: string;
}
export interface TraceCallersEngineItem {
    path: string;
    line: number;
    column: number | null;
    caller: string;
    callerSymbol: string | null;
    relation: string;
    language: PublicLanguage | null;
    snippet: string | null;
    receiverType: string | null;
}
export interface TraceCallersEngineRecursiveVia {
    relation: string | null;
    line: number | null;
    column: number | null;
    snippet: string | null;
}
export interface TraceCallersEngineRecursiveNode {
    key: string;
    path: string;
    symbol: string;
    depth: number;
    via: TraceCallersEngineRecursiveVia | null;
}
export interface TraceCallersEngineRecursivePathSegment {
    path: string;
    symbol: string;
    depth: number;
}
export interface TraceCallersEngineRecursiveCycle {
    fromKey: string;
    toKey: string;
    path: string[];
}
export interface TraceCallersEngineRecursiveTruncatedNode {
    key: string;
    path: string;
    symbol: string;
    depth: number;
}
export interface TraceCallersEngineProbableEntryPoint {
    key: string | null;
    path: string;
    symbol: string;
    depth: number | null;
    reasons: string[];
    probable: boolean | null;
    pathFromTarget: TraceCallersEngineRecursivePathSegment[];
}
export interface TraceCallersEngineClassificationRecord {
    path: string;
    symbol: string;
    caller: string;
    depth: number;
    line: number;
    column: number | null;
    relation: string;
    language: PublicLanguage | null;
    receiverType: string | null;
    snippet: string | null;
    calls: TraceCallersEngineTarget;
    pathFromTarget: TraceCallersEngineRecursivePathSegment[];
}
export interface TraceCallersEngineImplementationInterface {
    name: string | null;
    path: string | null;
    symbol: string | null;
}
export interface TraceCallersEngineImplementationReference {
    path: string;
    symbol: string | null;
}
export interface TraceCallersEngineImplementationInterfaceChain {
    kind: string;
    probable: boolean | null;
    interface: TraceCallersEngineImplementationInterface | null;
    implementation: TraceCallersEngineImplementationReference | null;
    implementations: TraceCallersEngineImplementationReference[];
    callers: TraceCallersEngineClassificationRecord[];
}
export interface TraceCallersEngineRecursiveSummary {
    directCallerCount: number;
    indirectCallerCount: number;
    probablePublicEntryPointCount: number;
    implementationInterfaceChainCount: number;
}
export interface TraceCallersEngineRecursiveClassifications {
    summary: TraceCallersEngineRecursiveSummary;
    directCallers: TraceCallersEngineClassificationRecord[];
    indirectCallers: TraceCallersEngineClassificationRecord[];
    probablePublicEntryPoints: TraceCallersEngineProbableEntryPoint[];
    implementationInterfaceChain: TraceCallersEngineImplementationInterfaceChain[];
}
export interface TraceCallersEngineRecursiveResult {
    enabled: boolean;
    root: TraceCallersEngineRecursiveNode;
    maxDepth: number;
    maxDepthObserved: number;
    nodeCount: number;
    edgeCount: number;
    pathCount: number;
    nodes: TraceCallersEngineRecursiveNode[];
    adjacency: Record<string, string[]>;
    paths: TraceCallersEngineRecursivePathSegment[][];
    cycles: TraceCallersEngineRecursiveCycle[];
    truncated: TraceCallersEngineRecursiveTruncatedNode[];
    probableEntryPoints: TraceCallersEngineProbableEntryPoint[];
    classifications: TraceCallersEngineRecursiveClassifications;
}
export interface TraceCallersEngineResult {
    resolvedPath: string | null;
    items: TraceCallersEngineItem[];
    totalMatched: number;
    truncated: boolean;
    recursive: TraceCallersEngineRecursiveResult | null;
}
export interface EngineSuccess<TResult = unknown> {
    id: string;
    ok: true;
    result: TResult;
}
export interface EngineFailure {
    id: string;
    ok: false;
    error: EngineError;
}
export type EngineResponse<TResult = unknown> = EngineSuccess<TResult> | EngineFailure;
export declare function nextRequestId(prefix?: string): string;
