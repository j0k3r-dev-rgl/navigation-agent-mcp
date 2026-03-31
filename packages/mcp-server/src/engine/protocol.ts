import type {
  MatchMode,
  PublicLanguage,
  PublicSymbolKind,
  PublicEndpointKind,
  PublicFramework,
} from "../contracts/public/code.ts";

export const ENGINE_CAPABILITIES = [
  "workspace.inspect_tree",
  "workspace.find_symbol",
  "workspace.list_endpoints",
] as const;
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

export type EngineResponse<TResult = unknown> =
  | EngineSuccess<TResult>
  | EngineFailure;

let requestSequence = 0;

export function nextRequestId(prefix = "req"): string {
  requestSequence += 1;
  return `${prefix}-${requestSequence}`;
}
