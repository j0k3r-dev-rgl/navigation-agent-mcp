import { normalizeTraceSymbolInput, } from "../contracts/public/code.js";
import { createResponseMeta, } from "../contracts/public/common.js";
import { nextRequestId, } from "../engine/protocol.js";
const TOOL_NAME = "code.trace_symbol";
export function createTraceSymbolService(options) {
    return {
        async execute(input) {
            let response;
            try {
                response = await options.engineClient.request({
                    id: nextRequestId(),
                    capability: "workspace.trace_symbol",
                    workspaceRoot: options.workspaceRoot,
                    payload: {
                        path: input.path,
                        symbol: input.symbol,
                        analyzerLanguage: resolveAnalyzerLanguage(input.language, input.framework),
                        publicLanguageFilter: resolveEffectiveLanguage(input.language, input.framework),
                    },
                });
            }
            catch (error) {
                return buildEngineFailureResponse(input, error);
            }
            if (!response.ok) {
                return buildMappedErrorResponse(input, response.error.code, response.error.message, response.error.details, response.error.retryable);
            }
            return buildSuccessResponse(input, response.result);
        },
        async validateAndExecute(payload) {
            const normalized = normalizeTraceSymbolInput(payload);
            if (!normalized.ok) {
                return buildValidationErrorResponse(normalized.issues);
            }
            return this.execute(normalized.value);
        },
    };
}
function buildSuccessResponse(input, result) {
    const entrypointPath = result.resolvedPath ?? input.path;
    const effectiveLanguage = resolveEffectiveLanguage(input.language, input.framework);
    const fileCount = result.items.length;
    return {
        tool: TOOL_NAME,
        status: result.truncated ? "partial" : "ok",
        summary: buildSummary(input.symbol, entrypointPath, result.totalMatched),
        data: {
            entrypoint: {
                path: entrypointPath,
                symbol: input.symbol,
                language: inferLanguageFromPath(entrypointPath),
            },
            fileCount,
            items: result.items.map((item) => ({
                path: item.path,
                language: item.language,
            })),
        },
        errors: [],
        meta: createResponseMeta({
            query: { ...input },
            resolvedPath: result.resolvedPath,
            truncated: result.truncated,
            counts: {
                returnedCount: fileCount,
                totalMatched: result.totalMatched,
            },
            detection: {
                effectiveLanguage,
                framework: input.framework ?? null,
            },
        }),
    };
}
function buildValidationErrorResponse(issues) {
    return {
        tool: TOOL_NAME,
        status: "error",
        summary: "Request validation failed.",
        data: emptyData(),
        errors: [
            {
                code: "INVALID_INPUT",
                message: "One or more input fields are invalid.",
                retryable: false,
                suggestion: "Correct the invalid fields and try again.",
                details: { issues },
            },
        ],
        meta: createResponseMeta({ query: {} }),
    };
}
function buildMappedErrorResponse(input, code, message, details, retryable) {
    const query = { ...input };
    if (code === "FILE_NOT_FOUND") {
        return {
            tool: TOOL_NAME,
            status: "error",
            summary: "Path not found.",
            data: emptyData(input.path, input.symbol),
            errors: [
                {
                    code,
                    message,
                    retryable,
                    suggestion: "Provide an existing file path inside the workspace root.",
                    details,
                },
            ],
            meta: createResponseMeta({ query }),
        };
    }
    if (code === "PATH_OUTSIDE_WORKSPACE") {
        return {
            tool: TOOL_NAME,
            status: "error",
            summary: "Path validation failed.",
            data: emptyData(input.path, input.symbol),
            errors: [
                {
                    code,
                    message,
                    retryable,
                    suggestion: "Use a file path inside the workspace root.",
                    details,
                },
            ],
            meta: createResponseMeta({ query }),
        };
    }
    if (code === "UNSUPPORTED_CAPABILITY") {
        return {
            tool: TOOL_NAME,
            status: "error",
            summary: "Symbol trace failed.",
            data: emptyData(input.path, input.symbol),
            errors: [
                {
                    code: "BACKEND_EXECUTION_FAILED",
                    message,
                    retryable,
                    suggestion: "Verify the engine supports workspace.trace_symbol and retry.",
                    details,
                },
            ],
            meta: createResponseMeta({ query }),
        };
    }
    return {
        tool: TOOL_NAME,
        status: "error",
        summary: "Symbol trace failed.",
        data: emptyData(input.path, input.symbol),
        errors: [
            {
                code,
                message,
                retryable,
                details,
            },
        ],
        meta: createResponseMeta({ query }),
    };
}
function buildEngineFailureResponse(input, error) {
    return buildMappedErrorResponse(input, "BACKEND_EXECUTION_FAILED", error instanceof Error ? error.message : String(error), {}, false);
}
function emptyData(path = "", symbol = "") {
    return {
        entrypoint: {
            path,
            symbol,
            language: inferLanguageFromPath(path),
        },
        fileCount: 0,
        items: [],
    };
}
function buildSummary(symbol, path, fileCount) {
    if (fileCount === 0) {
        return `Trace completed for '${symbol}' from '${path}' with no related files found.`;
    }
    if (fileCount === 1) {
        return `Traced 1 related file for '${symbol}' from '${path}'.`;
    }
    return `Traced ${fileCount} related files for '${symbol}' from '${path}'.`;
}
function resolveEffectiveLanguage(language, framework) {
    if (language) {
        return language;
    }
    if (framework === "react-router") {
        return "typescript";
    }
    if (framework === "spring") {
        return "java";
    }
    return null;
}
function resolveAnalyzerLanguage(language, framework) {
    const effective = resolveEffectiveLanguage(language, framework);
    if (effective === "java") {
        return "java";
    }
    if (effective === "python") {
        return "python";
    }
    if (effective === "rust") {
        return "rust";
    }
    return "typescript";
}
function inferLanguageFromPath(path) {
    const normalized = path.toLowerCase();
    if (normalized.endsWith(".ts") || normalized.endsWith(".tsx")) {
        return "typescript";
    }
    if (normalized.endsWith(".js") || normalized.endsWith(".jsx")) {
        return "javascript";
    }
    if (normalized.endsWith(".java")) {
        return "java";
    }
    if (normalized.endsWith(".py")) {
        return "python";
    }
    if (normalized.endsWith(".rs")) {
        return "rust";
    }
    return null;
}
