import { normalizeTraceFlowInput, } from "../contracts/public/code.js";
import { createResponseMeta, } from "../contracts/public/common.js";
import { nextRequestId, } from "../engine/protocol.js";
import { inferLanguageFromPath, resolveAnalyzerLanguage, resolveEffectiveLanguage, } from "./languageResolution.js";
const TOOL_NAME = "code.trace_flow";
export function createTraceFlowService(options) {
    return {
        async execute(input) {
            let response;
            try {
                response = await options.engineClient.request({
                    id: nextRequestId(),
                    capability: "workspace.trace_flow",
                    workspaceRoot: options.workspaceRoot,
                    payload: {
                        path: input.path,
                        symbol: input.symbol,
                        analyzerLanguage: resolveAnalyzerLanguage(input.language, input.framework, input.path),
                        publicLanguageFilter: resolveEffectiveLanguage(input.language, input.framework, input.path),
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
            const normalized = normalizeTraceFlowInput(payload);
            if (!normalized.ok) {
                return buildValidationErrorResponse(normalized.issues);
            }
            return this.execute(normalized.value);
        },
    };
}
function buildSuccessResponse(input, result) {
    const entrypointPath = result.resolvedPath ?? input.path;
    const effectiveLanguage = resolveEffectiveLanguage(input.language, input.framework, input.path);
    const calleeCount = countRootChildren(result.root);
    return {
        tool: TOOL_NAME,
        status: result.truncated ? "partial" : "ok",
        summary: buildSummary(input.symbol, entrypointPath, calleeCount),
        data: {
            entrypoint: {
                path: entrypointPath,
                symbol: input.symbol,
                language: inferLanguageFromPath(entrypointPath),
            },
            root: mapTraceFlowNode(result.root),
        },
        errors: [],
        meta: createResponseMeta({
            query: { ...input },
            resolvedPath: result.resolvedPath,
            truncated: result.truncated,
            counts: {
                returnedCount: calleeCount,
                totalMatched: calleeCount,
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
            summary: "Flow trace failed.",
            data: emptyData(input.path, input.symbol),
            errors: [
                {
                    code: "BACKEND_EXECUTION_FAILED",
                    message,
                    retryable,
                    suggestion: "Verify the engine supports workspace.trace_flow and retry.",
                    details,
                },
            ],
            meta: createResponseMeta({ query }),
        };
    }
    return {
        tool: TOOL_NAME,
        status: "error",
        summary: "Flow trace failed.",
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
        root: null,
    };
}
function buildSummary(symbol, path, calleeCount) {
    if (calleeCount === 0) {
        return `Trace completed for '${symbol}' from '${path}' with no callees found.`;
    }
    if (calleeCount === 1) {
        return `Traced 1 callee for '${symbol}' from '${path}'.`;
    }
    return `Traced ${calleeCount} callees for '${symbol}' from '${path}'.`;
}
function countRootChildren(root) {
    return root?.callers.length ?? 0;
}
function mapTraceFlowNode(node) {
    if (!node) {
        return null;
    }
    return {
        symbol: node.symbol,
        path: node.path,
        kind: node.kind,
        rangeLine: {
            init: node.rangeLine.init,
            end: node.rangeLine.end,
        },
        via: node.via ?? null,
        callers: node.callers.map((child) => mapTraceFlowNode(child)).filter(Boolean),
    };
}
