export const PUBLIC_LANGUAGES = ["typescript", "javascript", "go", "java", "python", "rust"] as const;
export type PublicLanguage = (typeof PUBLIC_LANGUAGES)[number];

export const PUBLIC_FRAMEWORKS = ["react-router", "spring"] as const;
export type PublicFramework = (typeof PUBLIC_FRAMEWORKS)[number];

export const PUBLIC_ENDPOINT_KINDS = ["any", "graphql", "rest", "route"] as const;
export type PublicEndpointKind = (typeof PUBLIC_ENDPOINT_KINDS)[number];

export const PUBLIC_SYMBOL_KINDS = [
  "any",
  "class",
  "interface",
  "function",
  "method",
  "type",
  "enum",
  "constructor",
  "annotation",
] as const;
export type PublicSymbolKind = (typeof PUBLIC_SYMBOL_KINDS)[number];

export const MATCH_MODES = ["exact", "fuzzy"] as const;
export type MatchMode = (typeof MATCH_MODES)[number];

export const CODE_TOOL_NAMES = [
  "code.inspect_tree",
  "code.list_endpoints",
  "code.find_symbol",
  "code.search_text",
  "code.trace_flow",
  "code.trace_callers",
] as const;

export type CodeToolName = (typeof CODE_TOOL_NAMES)[number];

export interface InspectTreeInput {
  path?: string | null;
  max_depth: number;
  extensions: string[];
  file_pattern?: string | null;
  include_stats: boolean;
  include_hidden: boolean;
}

export interface InspectTreeItemStats {
  sizeBytes: number;
  modifiedAt: string;
  isSymlink: boolean;
}

export interface InspectTreeItem {
  path: string;
  name: string;
  type: string;
  depth: number;
  extension?: string | null;
  stats?: InspectTreeItemStats | null;
}

export interface InspectTreeData {
  root: string;
  entryCount: number;
  items: InspectTreeItem[];
}

export interface FindSymbolInput {
  symbol: string;
  language?: PublicLanguage | null;
  framework?: PublicFramework | null;
  kind: PublicSymbolKind;
  match: MatchMode;
  path?: string | null;
  limit: number;
}

export interface PublicSymbolDefinition {
  symbol: string;
  kind: PublicSymbolKind;
  path: string;
  line: number;
  lineEnd: number;
  language: PublicLanguage | null;
}

export interface FindSymbolData {
  count: number;
  returnedCount: number;
  totalMatched: number;
  items: PublicSymbolDefinition[];
}

export interface ListEndpointsInput {
  path?: string | null;
  language?: PublicLanguage | null;
  framework?: PublicFramework | null;
  kind: PublicEndpointKind;
  limit: number;
}

export interface EndpointDefinition {
  name: string;
  kind: PublicEndpointKind;
  path: string | null;
  file: string;
  line: number;
  language: PublicLanguage | null;
  framework: PublicFramework | null;
}

export interface ListEndpointsCounts {
  byKind: Record<string, number>;
  byLanguage: Record<string, number>;
  byFramework: Record<string, number>;
}

export interface ListEndpointsData {
  totalCount: number;
  returnedCount: number;
  counts: ListEndpointsCounts;
  items: EndpointDefinition[];
}

export interface SearchTextInput {
  query: string;
  path?: string | null;
  language?: PublicLanguage | null;
  framework?: PublicFramework | null;
  include?: string | null;
  regex: boolean;
  context: number;
  limit: number;
}

export interface SearchTextSpan {
  colInit: number;
  colEnd: number;
}

export interface SearchTextMatch {
  line: number;
  spans: SearchTextSpan[];
}

export interface SearchTextFileMatch {
  path: string;
  language: PublicLanguage | null;
  matchCount: number;
  matches: SearchTextMatch[];
}

export interface SearchTextTopFile {
  path: string;
  language: PublicLanguage | null;
  matchCount: number;
}

export interface SearchTextData {
  fileCount: number;
  matchCount: number;
  totalFileCount: number;
  totalMatchCount: number;
  topFiles: SearchTextTopFile[];
  items: SearchTextFileMatch[];
}

export interface TraceFlowInput {
  path: string;
  symbol: string;
  language?: PublicLanguage | null;
  framework?: PublicFramework | null;
}

export interface TraceFlowEntrypoint {
  path: string;
  symbol: string;
  language: PublicLanguage | null;
}

export interface TraceFlowLineRange {
  init: number;
  end: number;
}

export interface TraceFlowVia {
  line: number;
  column: number | null;
  snippet: string | null;
  receiverType: string | null;
}

export interface TraceFlowNode {
  symbol: string;
  path: string;
  kind: string;
  rangeLine: TraceFlowLineRange;
  via: TraceFlowVia[] | null;
  callers: TraceFlowNode[];
}

export interface TraceFlowData {
  entrypoint: TraceFlowEntrypoint;
  root: TraceFlowNode | null;
}

export interface TraceCallersInput {
  path: string;
  symbol: string;
  language?: PublicLanguage | null;
  framework?: PublicFramework | null;
  recursive: boolean;
  max_depth?: number | null;
}

export interface TraceCallersTarget {
  path: string;
  symbol: string;
  language: PublicLanguage | null;
}

export interface TraceCallerRecord {
  path: string;
  line: number;
  column: number | null;
  caller: string;
  callerSymbol: string | null;
  callerRange: TraceCallersCallerRange;
  callSite: TraceCallersCallSite;
  calls: TraceCallersCallsTarget;
  relation: string;
  language: PublicLanguage | null;
  snippet: string | null;
  receiverType: string | null;
}

export interface TraceCallersCallerRange {
  startLine: number;
  endLine: number;
}

export interface TraceCallersCallSite {
  line: number;
  column: number | null;
  relation: string;
  snippet: string | null;
  receiverType: string | null;
}

export interface TraceCallersRecursiveVia {
  relation: string | null;
  line: number | null;
  column: number | null;
  snippet: string | null;
}

export interface TraceCallersRecursiveNode {
  key: string;
  path: string;
  symbol: string;
  depth: number;
  via: TraceCallersRecursiveVia | null;
}

export interface TraceCallersRecursivePathSegment {
  path: string;
  symbol: string;
  depth: number;
}

export interface TraceCallersRecursiveCycle {
  fromKey: string;
  toKey: string;
  path: string[];
}

export interface TraceCallersRecursiveTruncatedNode {
  key: string;
  path: string;
  symbol: string;
  depth: number;
}

export interface TraceCallersProbableEntryPoint {
  key: string | null;
  path: string;
  symbol: string;
  depth: number | null;
  reasons: string[];
  probable: boolean | null;
  pathFromTarget: TraceCallersRecursivePathSegment[];
}

export interface TraceCallersCallsTarget {
  path: string;
  symbol: string;
}

export interface TraceCallersClassificationRecord {
  path: string;
  symbol: string;
  caller: string;
  callerSymbol: string | null;
  callerRange: TraceCallersCallerRange;
  callSite: TraceCallersCallSite;
  depth: number;
  line: number;
  column: number | null;
  relation: string;
  language: PublicLanguage | null;
  receiverType: string | null;
  snippet: string | null;
  calls: TraceCallersCallsTarget;
  pathFromTarget: TraceCallersRecursivePathSegment[];
}

export interface TraceCallersImplementationInterface {
  name: string | null;
  path: string | null;
  symbol: string | null;
}

export interface TraceCallersImplementationReference {
  path: string;
  symbol: string | null;
}

export interface TraceCallersImplementationInterfaceChain {
  kind: string;
  probable: boolean | null;
  interface: TraceCallersImplementationInterface | null;
  implementation: TraceCallersImplementationReference | null;
  implementations: TraceCallersImplementationReference[];
  callers: TraceCallersClassificationRecord[];
}

export interface TraceCallersRecursiveSummary {
  directCallerCount: number;
  indirectCallerCount: number;
  probablePublicEntryPointCount: number;
  implementationInterfaceChainCount: number;
}

export interface TraceCallersRecursiveClassifications {
  summary: TraceCallersRecursiveSummary;
  directCallers: TraceCallersClassificationRecord[];
  indirectCallers: TraceCallersClassificationRecord[];
  probablePublicEntryPoints: TraceCallersProbableEntryPoint[];
  implementationInterfaceChain: TraceCallersImplementationInterfaceChain[];
}

export interface TraceCallersRecursiveData {
  enabled: boolean;
  root: TraceCallersRecursiveNode;
  maxDepth: number;
  maxDepthObserved: number;
  nodeCount: number;
  edgeCount: number;
  pathCount: number;
  nodes: TraceCallersRecursiveNode[];
  adjacency: Record<string, string[]>;
  paths: TraceCallersRecursivePathSegment[][];
  cycles: TraceCallersRecursiveCycle[];
  truncated: TraceCallersRecursiveTruncatedNode[];
  probableEntryPoints: TraceCallersProbableEntryPoint[];
  classifications: TraceCallersRecursiveClassifications;
}

export interface TraceCallersData {
  target: TraceCallersTarget;
  count: number;
  returnedCount: number;
  items: TraceCallerRecord[];
  recursive: TraceCallersRecursiveData | null;
}

export interface ValidationIssue {
  field: string;
  message: string;
  type: string;
}

const sharedDefs = {
  PublicLanguage: {
    type: "string",
    enum: [...PUBLIC_LANGUAGES],
  },
  PublicFramework: {
    type: "string",
    enum: [...PUBLIC_FRAMEWORKS],
  },
  PublicEndpointKind: {
    type: "string",
    enum: [...PUBLIC_ENDPOINT_KINDS],
  },
  PublicSymbolKind: {
    type: "string",
    enum: [...PUBLIC_SYMBOL_KINDS],
  },
  MatchMode: {
    type: "string",
    enum: [...MATCH_MODES],
  },
} as const;

export const inspectTreeInputSchema = {
  type: "object",
  properties: {
    path: {
      anyOf: [{ type: "string" }, { type: "null" }],
      default: null,
      description: "Optional workspace-relative or absolute file/directory scope.",
    },
    max_depth: {
      type: "integer",
      default: 3,
      minimum: 0,
      maximum: 20,
      description: "Maximum depth relative to the resolved scope root.",
    },
    extensions: {
      anyOf: [
        {
          type: "array",
          items: { type: "string" },
        },
        { type: "null" },
      ],
      default: null,
      description: "Optional file extension filter such as ['.py', '.ts']. Directories remain visible.",
    },
    file_pattern: {
      anyOf: [{ type: "string" }, { type: "null" }],
      default: null,
      description: "Optional filename glob such as '*.py'.",
    },
    include_stats: {
      type: "boolean",
      default: false,
      description: "Include size, modified time, and symlink metadata.",
    },
    include_hidden: {
      type: "boolean",
      default: false,
      description: "Include hidden entries except the hard ignore list.",
    },
  },
  required: [],
} as const;

export const listEndpointsInputSchema = {
  type: "object",
  properties: {
    path: {
      anyOf: [{ type: "string" }, { type: "null" }],
      default: null,
    },
    language: {
      anyOf: [{ $ref: "#/$defs/PublicLanguage" }, { type: "null" }],
      default: null,
    },
    framework: {
      anyOf: [{ $ref: "#/$defs/PublicFramework" }, { type: "null" }],
      default: null,
    },
    kind: {
      allOf: [{ $ref: "#/$defs/PublicEndpointKind" }],
      default: "any",
    },
    limit: {
      type: "integer",
      default: 50,
      minimum: 1,
      maximum: 200,
    },
  },
  required: [],
  $defs: sharedDefs,
} as const;

export const findSymbolInputSchema = {
  type: "object",
  properties: {
    symbol: {
      type: "string",
      minLength: 1,
    },
    language: {
      anyOf: [{ $ref: "#/$defs/PublicLanguage" }, { type: "null" }],
      default: null,
    },
    framework: {
      anyOf: [{ $ref: "#/$defs/PublicFramework" }, { type: "null" }],
      default: null,
    },
    kind: {
      allOf: [{ $ref: "#/$defs/PublicSymbolKind" }],
      default: "any",
    },
    match: {
      allOf: [{ $ref: "#/$defs/MatchMode" }],
      default: "exact",
    },
    path: {
      anyOf: [{ type: "string" }, { type: "null" }],
      default: null,
    },
    limit: {
      type: "integer",
      default: 50,
      minimum: 1,
      maximum: 200,
    },
  },
  required: ["symbol"],
  $defs: sharedDefs,
} as const;

export const searchTextInputSchema = {
  type: "object",
  properties: {
    query: {
      type: "string",
      minLength: 1,
    },
    path: {
      anyOf: [{ type: "string" }, { type: "null" }],
      default: null,
    },
    language: {
      anyOf: [{ $ref: "#/$defs/PublicLanguage" }, { type: "null" }],
      default: null,
    },
    framework: {
      anyOf: [{ $ref: "#/$defs/PublicFramework" }, { type: "null" }],
      default: null,
    },
    include: {
      anyOf: [{ type: "string" }, { type: "null" }],
      default: null,
    },
    regex: {
      type: "boolean",
      default: false,
    },
    context: {
      type: "integer",
      default: 1,
      minimum: 0,
      maximum: 10,
    },
    limit: {
      type: "integer",
      default: 50,
      minimum: 1,
      maximum: 200,
    },
  },
  required: ["query"],
  $defs: sharedDefs,
} as const;

export const traceFlowInputSchema = {
  type: "object",
  properties: {
    path: { type: "string", minLength: 1 },
    symbol: { type: "string", minLength: 1 },
    language: {
      anyOf: [{ $ref: "#/$defs/PublicLanguage" }, { type: "null" }],
      default: null,
    },
    framework: {
      anyOf: [{ $ref: "#/$defs/PublicFramework" }, { type: "null" }],
      default: null,
    },
  },
  required: ["path", "symbol"],
  $defs: sharedDefs,
} as const;

export const traceCallersInputSchema = {
  type: "object",
  properties: {
    path: { type: "string", minLength: 1 },
    symbol: { type: "string", minLength: 1 },
    language: {
      anyOf: [{ $ref: "#/$defs/PublicLanguage" }, { type: "null" }],
      default: null,
    },
    framework: {
      anyOf: [{ $ref: "#/$defs/PublicFramework" }, { type: "null" }],
      default: null,
    },
    recursive: {
      type: "boolean",
      default: false,
    },
    max_depth: {
      anyOf: [
        {
          type: "integer",
          minimum: 1,
          maximum: 8,
        },
        {
          type: "null"
        }
      ],
      default: null,
    },
  },
  required: ["path", "symbol"],
  $defs: sharedDefs,
} as const;

export const codeToolSchemas = {
  "code.inspect_tree": inspectTreeInputSchema,
  "code.list_endpoints": listEndpointsInputSchema,
  "code.find_symbol": findSymbolInputSchema,
  "code.search_text": searchTextInputSchema,
  "code.trace_flow": traceFlowInputSchema,
  "code.trace_callers": traceCallersInputSchema,
} as const;

export function normalizeInspectTreeInput(
  payload: Record<string, unknown>,
): { ok: true; value: InspectTreeInput } | { ok: false; issues: ValidationIssue[] } {
  const issues: ValidationIssue[] = [];

  const path = normalizeOptionalString(payload.path, "path", issues);
  const filePattern = normalizeOptionalString(
    payload.file_pattern,
    "file_pattern",
    issues,
  );
  const maxDepth = normalizeInteger(payload.max_depth, "max_depth", 3, 0, 20, issues);
  const includeStats = normalizeBoolean(
    payload.include_stats,
    "include_stats",
    false,
    issues,
  );
  const includeHidden = normalizeBoolean(
    payload.include_hidden,
    "include_hidden",
    false,
    issues,
  );
  const extensions = normalizeExtensions(payload.extensions, issues);

  if (issues.length > 0) {
    return { ok: false, issues };
  }

  return {
    ok: true,
    value: {
      path,
      max_depth: maxDepth,
      extensions,
      file_pattern: filePattern,
      include_stats: includeStats,
      include_hidden: includeHidden,
    },
  };
}

export function normalizeFindSymbolInput(
  payload: Record<string, unknown>,
): { ok: true; value: FindSymbolInput } | { ok: false; issues: ValidationIssue[] } {
  const issues: ValidationIssue[] = [];

  const symbol = normalizeRequiredTrimmedString(payload.symbol, "symbol", issues);
  const language = normalizeEnumValue(
    payload.language,
    "language",
    PUBLIC_LANGUAGES,
    issues,
  );
  const framework = normalizeEnumValue(
    payload.framework,
    "framework",
    PUBLIC_FRAMEWORKS,
    issues,
  );
  const kind =
    normalizeEnumValue(payload.kind, "kind", PUBLIC_SYMBOL_KINDS, issues) ?? "any";
  const match =
    normalizeEnumValue(payload.match, "match", MATCH_MODES, issues) ?? "exact";
  const scopedPath = normalizeOptionalString(payload.path, "path", issues);
  const limit = normalizeInteger(payload.limit, "limit", 50, 1, 200, issues);

  if (issues.length > 0) {
    return { ok: false, issues };
  }

  return {
    ok: true,
    value: {
      symbol,
      language,
      framework,
      kind,
      match,
      path: scopedPath,
      limit,
    },
  };
}

export function normalizeListEndpointsInput(
  payload: Record<string, unknown>,
): { ok: true; value: ListEndpointsInput } | { ok: false; issues: ValidationIssue[] } {
  const issues: ValidationIssue[] = [];

  const scopedPath = normalizeOptionalString(payload.path, "path", issues);
  const language = normalizeEnumValue(
    payload.language,
    "language",
    PUBLIC_LANGUAGES,
    issues,
  );
  const framework = normalizeEnumValue(
    payload.framework,
    "framework",
    PUBLIC_FRAMEWORKS,
    issues,
  );
  const kind =
    normalizeEnumValue(payload.kind, "kind", PUBLIC_ENDPOINT_KINDS, issues) ?? "any";
  const limit = normalizeInteger(payload.limit, "limit", 50, 1, 200, issues);

  if (issues.length > 0) {
    return { ok: false, issues };
  }

  return {
    ok: true,
    value: {
      path: scopedPath,
      language,
      framework,
      kind,
      limit,
    },
  };
}

export function normalizeSearchTextInput(
  payload: Record<string, unknown>,
): { ok: true; value: SearchTextInput } | { ok: false; issues: ValidationIssue[] } {
  const issues: ValidationIssue[] = [];

  const query = normalizeRequiredTrimmedString(payload.query, "query", issues);
  const scopedPath = normalizeOptionalString(payload.path, "path", issues);
  const language = normalizeEnumValue(
    payload.language,
    "language",
    PUBLIC_LANGUAGES,
    issues,
  );
  const framework = normalizeEnumValue(
    payload.framework,
    "framework",
    PUBLIC_FRAMEWORKS,
    issues,
  );
  const include = normalizeOptionalString(payload.include, "include", issues);
  const regex = normalizeBoolean(payload.regex, "regex", false, issues);
  const context = normalizeInteger(payload.context, "context", 1, 0, 10, issues);
  const limit = normalizeInteger(payload.limit, "limit", 50, 1, 200, issues);

  if (issues.length > 0) {
    return { ok: false, issues };
  }

  return {
    ok: true,
    value: {
      query,
      path: scopedPath,
      language,
      framework,
      include,
      regex,
      context,
      limit,
    },
  };
}

export function normalizeTraceFlowInput(
  payload: Record<string, unknown>,
): { ok: true; value: TraceFlowInput } | { ok: false; issues: ValidationIssue[] } {
  const issues: ValidationIssue[] = [];

  const scopedPath = normalizeRequiredTrimmedString(payload.path, "path", issues);
  const symbol = normalizeRequiredTrimmedString(payload.symbol, "symbol", issues);
  const language = normalizeEnumValue(
    payload.language,
    "language",
    PUBLIC_LANGUAGES,
    issues,
  );
  const framework = normalizeEnumValue(
    payload.framework,
    "framework",
    PUBLIC_FRAMEWORKS,
    issues,
  );

  if (issues.length > 0) {
    return { ok: false, issues };
  }

  return {
    ok: true,
    value: {
      path: scopedPath,
      symbol,
      language,
      framework,
    },
  };
}

export function normalizeTraceCallersInput(
  payload: Record<string, unknown>,
): { ok: true; value: TraceCallersInput } | { ok: false; issues: ValidationIssue[] } {
  const issues: ValidationIssue[] = [];

  const scopedPath = normalizeRequiredTrimmedString(payload.path, "path", issues);
  const symbol = normalizeRequiredTrimmedString(payload.symbol, "symbol", issues);
  const language = normalizeEnumValue(
    payload.language,
    "language",
    PUBLIC_LANGUAGES,
    issues,
  );
  const framework = normalizeEnumValue(
    payload.framework,
    "framework",
    PUBLIC_FRAMEWORKS,
    issues,
  );
  const recursive = normalizeBoolean(payload.recursive, "recursive", false, issues);
  const maxDepth = normalizeNullableInteger(payload.max_depth, "max_depth", 1, 8, issues);

  if (issues.length > 0) {
    return { ok: false, issues };
  }

  return {
    ok: true,
    value: {
      path: scopedPath,
      symbol,
      language,
      framework,
      recursive,
      max_depth: maxDepth,
    },
  };
}

function normalizeRequiredTrimmedString(
  value: unknown,
  field: string,
  issues: ValidationIssue[],
): string {
  if (typeof value !== "string") {
    issues.push({ field, message: "Input should be a valid string.", type: "string_type" });
    return "";
  }

  const normalized = value.trim();
  if (!normalized) {
    issues.push({
      field,
      message: "String should have at least 1 character.",
      type: "string_too_short",
    });
  }

  return normalized;
}

function normalizeOptionalString(
  value: unknown,
  field: string,
  issues: ValidationIssue[],
): string | null {
  if (value === undefined || value === null) {
    return null;
  }
  if (typeof value !== "string") {
    issues.push({ field, message: "Input should be a valid string.", type: "string_type" });
    return null;
  }
  const normalized = value.trim();
  return normalized.length > 0 ? normalized : null;
}

function normalizeInteger(
  value: unknown,
  field: string,
  fallback: number,
  min: number,
  max: number,
  issues: ValidationIssue[],
): number {
  if (value === undefined || value === null) {
    return fallback;
  }
  if (typeof value !== "number" || !Number.isInteger(value)) {
    issues.push({ field, message: "Input should be a valid integer.", type: "int_type" });
    return fallback;
  }
  if (value < min || value > max) {
    issues.push({
      field,
      message: `Input should be greater than or equal to ${min} and less than or equal to ${max}.`,
      type: "range_error",
    });
    return fallback;
  }
  return value;
}

function normalizeNullableInteger(
  value: unknown,
  field: string,
  min: number,
  max: number,
  issues: ValidationIssue[],
): number | null {
  if (value === undefined || value === null) {
    return null;
  }
  if (typeof value !== "number" || !Number.isInteger(value)) {
    issues.push({ field, message: "Input should be a valid integer.", type: "int_type" });
    return null;
  }
  if (value < min || value > max) {
    issues.push({
      field,
      message: `Input should be greater than or equal to ${min} and less than or equal to ${max}.`,
      type: "range_error",
    });
    return null;
  }
  return value;
}

function normalizeBoolean(
  value: unknown,
  field: string,
  fallback: boolean,
  issues: ValidationIssue[],
): boolean {
  if (value === undefined || value === null) {
    return fallback;
  }
  if (typeof value !== "boolean") {
    issues.push({ field, message: "Input should be a valid boolean.", type: "bool_type" });
    return fallback;
  }
  return value;
}

function normalizeEnumValue<TValue extends string>(
  value: unknown,
  field: string,
  allowed: readonly TValue[],
  issues: ValidationIssue[],
): TValue | null {
  if (value === undefined || value === null) {
    return null;
  }
  if (typeof value !== "string") {
    issues.push({ field, message: "Input should be a valid string.", type: "string_type" });
    return null;
  }
  if (!(allowed as readonly string[]).includes(value)) {
    issues.push({
      field,
      message: `Input should be one of: ${allowed.join(", ")}.`,
      type: "enum",
    });
    return null;
  }
  return value as TValue;
}

function normalizeExtensions(
  value: unknown,
  issues: ValidationIssue[],
): string[] {
  if (value === undefined || value === null) {
    return [];
  }
  if (!Array.isArray(value)) {
    issues.push({
      field: "extensions",
      message: "extensions must be a list of strings",
      type: "list_type",
    });
    return [];
  }

  const normalized = new Set<string>();
  for (const item of value) {
    if (typeof item !== "string") {
      issues.push({
        field: "extensions",
        message: "extensions must contain only strings",
        type: "string_type",
      });
      continue;
    }

    const trimmed = item.trim().toLowerCase();
    if (!trimmed) {
      continue;
    }
    if (trimmed === ".") {
      issues.push({
        field: "extensions",
        message: "extensions entries must not be '.'",
        type: "value_error",
      });
      continue;
    }

    normalized.add(trimmed.startsWith(".") ? trimmed : `.${trimmed}`);
  }

  return [...normalized].sort();
}
