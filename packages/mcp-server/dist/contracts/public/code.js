export const PUBLIC_LANGUAGES = ["typescript", "javascript", "go", "java", "php", "python", "rust", "csharp"];
export const PUBLIC_FRAMEWORKS = ["react-router", "spring"];
export const PUBLIC_ENDPOINT_KINDS = ["any", "graphql", "rest", "route"];
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
];
export const MATCH_MODES = ["exact", "fuzzy"];
export const CODE_TOOL_NAMES = [
    "code.inspect_tree",
    "code.list_endpoints",
    "code.find_symbol",
    "code.search_text",
    "code.trace_flow",
    "code.trace_callers",
];
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
};
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
};
export const listEndpointsInputSchema = {
    type: "object",
    description: "Workspace-only route surface discovery. Use this to enumerate framework-detectable public routes, REST endpoints, and GraphQL resolvers; use find_symbol or trace tools when you need symbol-level flow or impact analysis.",
    properties: {
        path: {
            anyOf: [{ type: "string" }, { type: "null" }],
            default: null,
            description: "Optional workspace-relative scope for endpoint discovery. Use this to audit a module, feature area, or app subtree instead of the whole workspace.",
        },
        language: {
            anyOf: [{ $ref: "#/$defs/PublicLanguage" }, { type: "null" }],
            default: null,
            description: "Optional language hint for endpoint detection. Useful when the workspace contains multiple stacks.",
        },
        framework: {
            anyOf: [{ $ref: "#/$defs/PublicFramework" }, { type: "null" }],
            default: null,
            description: "Optional framework hint such as spring or react-router. This is often the most important filter because endpoint detection depends on framework-specific patterns.",
        },
        kind: {
            allOf: [{ $ref: "#/$defs/PublicEndpointKind" }],
            default: "any",
            description: "Endpoint surface filter. Use rest, graphql, or route when you want a narrower public entrypoint inventory.",
        },
        limit: {
            type: "integer",
            default: 50,
            minimum: 1,
            maximum: 200,
            description: "Maximum number of detected entrypoints to return. Lower this when scoping tightly; raise it for broader audits of the route surface.",
        },
    },
    required: [],
    $defs: sharedDefs,
};
export const findSymbolInputSchema = {
    type: "object",
    description: "Workspace-only symbol lookup. Use this first when you know the symbol name but still need the defining file path before running code.trace_callers or code.trace_flow.",
    properties: {
        symbol: {
            type: "string",
            minLength: 1,
            description: "Symbol name to resolve in the current workspace. Use exact names when possible to get the best trace entrypoint.",
        },
        language: {
            anyOf: [{ $ref: "#/$defs/PublicLanguage" }, { type: "null" }],
            default: null,
            description: "Optional language hint for narrowing symbol lookup inside the workspace.",
        },
        framework: {
            anyOf: [{ $ref: "#/$defs/PublicFramework" }, { type: "null" }],
            default: null,
            description: "Optional framework hint such as react-router or spring to improve framework-aware symbol matching.",
        },
        kind: {
            allOf: [{ $ref: "#/$defs/PublicSymbolKind" }],
            default: "any",
            description: "Optional symbol kind filter. Narrow this when you know whether the target is a class, function, method, or other specific symbol type.",
        },
        match: {
            allOf: [{ $ref: "#/$defs/MatchMode" }],
            default: "exact",
            description: "Match strategy for symbol lookup. Prefer exact for precise tracing workflows; use fuzzy only when the exact symbol name is uncertain.",
        },
        path: {
            anyOf: [{ type: "string" }, { type: "null" }],
            default: null,
            description: "Optional workspace-relative scope to narrow lookup before tracing or reading files.",
        },
        limit: {
            type: "integer",
            default: 50,
            minimum: 1,
            maximum: 200,
            description: "Maximum number of matching definitions to return. Lower this when you already have a narrow scope and want a faster trace handoff.",
        },
    },
    required: ["symbol"],
    $defs: sharedDefs,
};
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
};
export const traceFlowInputSchema = {
    type: "object",
    description: "Workspace-only forward trace for a known symbol. Use this after code.find_symbol when you need downstream flow before modifying a function, method, class entrypoint, or route handler.",
    properties: {
        path: {
            type: "string",
            minLength: 1,
            description: "File inside the current workspace where the target symbol is defined. Normally obtain this path from code.find_symbol before tracing.",
        },
        symbol: {
            type: "string",
            minLength: 1,
            description: "Exact symbol to trace forward from the given file. Use this to inspect what the symbol calls or touches downstream.",
        },
        language: {
            anyOf: [{ $ref: "#/$defs/PublicLanguage" }, { type: "null" }],
            default: null,
            description: "Optional language hint for analyzer selection. Leave null to infer from framework or path when possible.",
        },
        framework: {
            anyOf: [{ $ref: "#/$defs/PublicFramework" }, { type: "null" }],
            default: null,
            description: "Optional framework hint such as react-router or spring to improve symbol resolution in framework-aware projects.",
        },
    },
    required: ["path", "symbol"],
    $defs: sharedDefs,
};
export const traceCallersInputSchema = {
    type: "object",
    description: "Workspace-only reverse trace for a known symbol. Use this after code.find_symbol for impact analysis before changing signatures, renaming functions, or modifying shared behavior.",
    properties: {
        path: {
            type: "string",
            minLength: 1,
            description: "File inside the current workspace where the target symbol is defined. Normally obtain this path from code.find_symbol before tracing callers.",
        },
        symbol: {
            type: "string",
            minLength: 1,
            description: "Exact symbol to trace backward from the given file. Use this to learn who calls the symbol before changing or removing it.",
        },
        language: {
            anyOf: [{ $ref: "#/$defs/PublicLanguage" }, { type: "null" }],
            default: null,
            description: "Optional language hint for analyzer selection. Leave null to infer from framework or path when possible.",
        },
        framework: {
            anyOf: [{ $ref: "#/$defs/PublicFramework" }, { type: "null" }],
            default: null,
            description: "Optional framework hint such as react-router or spring to improve symbol resolution in framework-aware projects.",
        },
        recursive: {
            type: "boolean",
            default: false,
            description: "When true, continue reverse tracing beyond direct callers to build a broader impact tree across the workspace.",
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
            description: "Optional recursion limit for recursive caller tracing. Lower values are useful when you want impact analysis without a large response.",
        },
    },
    required: ["path", "symbol"],
    $defs: sharedDefs,
};
export const codeToolSchemas = {
    "code.inspect_tree": inspectTreeInputSchema,
    "code.list_endpoints": listEndpointsInputSchema,
    "code.find_symbol": findSymbolInputSchema,
    "code.search_text": searchTextInputSchema,
    "code.trace_flow": traceFlowInputSchema,
    "code.trace_callers": traceCallersInputSchema,
};
export function normalizeInspectTreeInput(payload) {
    const issues = [];
    const path = normalizeOptionalString(payload.path, "path", issues);
    const filePattern = normalizeOptionalString(payload.file_pattern, "file_pattern", issues);
    const maxDepth = normalizeInteger(payload.max_depth, "max_depth", 3, 0, 20, issues);
    const includeStats = normalizeBoolean(payload.include_stats, "include_stats", false, issues);
    const includeHidden = normalizeBoolean(payload.include_hidden, "include_hidden", false, issues);
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
export function normalizeFindSymbolInput(payload) {
    const issues = [];
    const symbol = normalizeRequiredTrimmedString(payload.symbol, "symbol", issues);
    const language = normalizeEnumValue(payload.language, "language", PUBLIC_LANGUAGES, issues);
    const framework = normalizeEnumValue(payload.framework, "framework", PUBLIC_FRAMEWORKS, issues);
    const kind = normalizeEnumValue(payload.kind, "kind", PUBLIC_SYMBOL_KINDS, issues) ?? "any";
    const match = normalizeEnumValue(payload.match, "match", MATCH_MODES, issues) ?? "exact";
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
export function normalizeListEndpointsInput(payload) {
    const issues = [];
    const scopedPath = normalizeOptionalString(payload.path, "path", issues);
    const language = normalizeEnumValue(payload.language, "language", PUBLIC_LANGUAGES, issues);
    const framework = normalizeEnumValue(payload.framework, "framework", PUBLIC_FRAMEWORKS, issues);
    const kind = normalizeEnumValue(payload.kind, "kind", PUBLIC_ENDPOINT_KINDS, issues) ?? "any";
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
export function normalizeSearchTextInput(payload) {
    const issues = [];
    const query = normalizeRequiredTrimmedString(payload.query, "query", issues);
    const scopedPath = normalizeOptionalString(payload.path, "path", issues);
    const language = normalizeEnumValue(payload.language, "language", PUBLIC_LANGUAGES, issues);
    const framework = normalizeEnumValue(payload.framework, "framework", PUBLIC_FRAMEWORKS, issues);
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
export function normalizeTraceFlowInput(payload) {
    const issues = [];
    const scopedPath = normalizeRequiredTrimmedString(payload.path, "path", issues);
    const symbol = normalizeRequiredTrimmedString(payload.symbol, "symbol", issues);
    const language = normalizeEnumValue(payload.language, "language", PUBLIC_LANGUAGES, issues);
    const framework = normalizeEnumValue(payload.framework, "framework", PUBLIC_FRAMEWORKS, issues);
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
export function normalizeTraceCallersInput(payload) {
    const issues = [];
    const scopedPath = normalizeRequiredTrimmedString(payload.path, "path", issues);
    const symbol = normalizeRequiredTrimmedString(payload.symbol, "symbol", issues);
    const language = normalizeEnumValue(payload.language, "language", PUBLIC_LANGUAGES, issues);
    const framework = normalizeEnumValue(payload.framework, "framework", PUBLIC_FRAMEWORKS, issues);
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
function normalizeRequiredTrimmedString(value, field, issues) {
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
function normalizeOptionalString(value, field, issues) {
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
function normalizeInteger(value, field, fallback, min, max, issues) {
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
function normalizeNullableInteger(value, field, min, max, issues) {
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
function normalizeBoolean(value, field, fallback, issues) {
    if (value === undefined || value === null) {
        return fallback;
    }
    if (typeof value !== "boolean") {
        issues.push({ field, message: "Input should be a valid boolean.", type: "bool_type" });
        return fallback;
    }
    return value;
}
function normalizeEnumValue(value, field, allowed, issues) {
    if (value === undefined || value === null) {
        return null;
    }
    if (typeof value !== "string") {
        issues.push({ field, message: "Input should be a valid string.", type: "string_type" });
        return null;
    }
    if (!allowed.includes(value)) {
        issues.push({
            field,
            message: `Input should be one of: ${allowed.join(", ")}.`,
            type: "enum",
        });
        return null;
    }
    return value;
}
function normalizeExtensions(value, issues) {
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
    const normalized = new Set();
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
