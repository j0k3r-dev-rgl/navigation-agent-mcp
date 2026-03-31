import { normalizeInspectTreeInput, } from "../contracts/public/code.js";
import { createResponseMeta, } from "../contracts/public/common.js";
import { nextRequestId, } from "../engine/protocol.js";
export function createInspectTreeService(options) {
    return {
        async execute(input) {
            let response;
            try {
                response = await options.engineClient.request({
                    id: nextRequestId(),
                    capability: "workspace.inspect_tree",
                    workspaceRoot: options.workspaceRoot,
                    payload: {
                        path: input.path ?? null,
                        maxDepth: input.max_depth,
                        extensions: input.extensions,
                        filePattern: input.file_pattern ?? null,
                        includeStats: input.include_stats,
                        includeHidden: input.include_hidden,
                    },
                });
            }
            catch (error) {
                return buildEngineFailureResponse(input, error);
            }
            if (!response.ok) {
                return buildMappedErrorResponse(input, response.error.code, response.error.message, response.error.details, response.error.retryable);
            }
            return {
                tool: "code.inspect_tree",
                status: response.result.truncated ? "partial" : "ok",
                summary: buildSummary(response.result.root, response.result.items.length, response.result.truncated),
                data: mapInspectTreeResult(response.result),
                errors: response.result.truncated
                    ? [
                        {
                            code: "RESULT_TRUNCATED",
                            message: `Tree inspection hit the safety cap of ${response.result.maxItems} items.`,
                            retryable: false,
                            details: {
                                returned: response.result.items.length,
                                maxItems: response.result.maxItems,
                            },
                        },
                    ]
                    : [],
                meta: createResponseMeta({
                    query: { ...input },
                    resolvedPath: response.result.root,
                    truncated: response.result.truncated,
                    counts: {
                        returnedCount: response.result.items.length,
                        totalMatched: response.result.truncated
                            ? null
                            : response.result.items.length,
                    },
                    detection: {
                        includeHidden: input.include_hidden ? "true" : "false",
                        stats: input.include_stats ? "true" : "false",
                    },
                }),
            };
        },
        async validateAndExecute(payload) {
            const normalized = normalizeInspectTreeInput(payload);
            if (!normalized.ok) {
                return buildValidationErrorResponse(normalized.issues);
            }
            return this.execute(normalized.value);
        },
    };
}
function mapInspectTreeResult(result) {
    return {
        root: result.root,
        entryCount: result.items.length,
        items: result.items.map((item) => ({
            path: item.path,
            name: item.name,
            type: item.type,
            depth: item.depth,
            extension: item.extension ?? null,
            stats: item.stats ?? null,
        })),
    };
}
function buildValidationErrorResponse(issues) {
    return {
        tool: "code.inspect_tree",
        status: "error",
        summary: "Request validation failed.",
        data: { root: ".", entryCount: 0, items: [] },
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
            tool: "code.inspect_tree",
            status: "error",
            summary: "Path not found.",
            data: { root: ".", entryCount: 0, items: [] },
            errors: [
                {
                    code,
                    message,
                    retryable,
                    suggestion: "Provide an existing file or directory path inside the workspace root.",
                    details,
                },
            ],
            meta: createResponseMeta({ query }),
        };
    }
    if (code === "PATH_OUTSIDE_WORKSPACE") {
        return {
            tool: "code.inspect_tree",
            status: "error",
            summary: "Path validation failed.",
            data: { root: ".", entryCount: 0, items: [] },
            errors: [
                {
                    code,
                    message,
                    retryable,
                    suggestion: "Use a path inside the workspace root or omit the path filter.",
                    details,
                },
            ],
            meta: createResponseMeta({ query }),
        };
    }
    if (code === "UNSUPPORTED_CAPABILITY") {
        return {
            tool: "code.inspect_tree",
            status: "error",
            summary: "Inspect tree execution failed.",
            data: { root: input.path ?? ".", entryCount: 0, items: [] },
            errors: [
                {
                    code: "BACKEND_EXECUTION_FAILED",
                    message,
                    retryable,
                    suggestion: "Verify the engine supports workspace.inspect_tree and retry.",
                    details,
                },
            ],
            meta: createResponseMeta({ query }),
        };
    }
    return {
        tool: "code.inspect_tree",
        status: "error",
        summary: "Inspect tree execution failed.",
        data: { root: input.path ?? ".", entryCount: 0, items: [] },
        errors: [
            {
                code: code === "BACKEND_EXECUTION_FAILED" ? code : "BACKEND_EXECUTION_FAILED",
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
function buildSummary(root, entryCount, truncated) {
    if (entryCount === 0) {
        return `No tree entries found under '${root}'.`;
    }
    if (truncated) {
        return `Inspected ${entryCount} tree entries under '${root}' and returned a truncated subset.`;
    }
    if (entryCount === 1) {
        return `Inspected 1 tree entry under '${root}'.`;
    }
    return `Inspected ${entryCount} tree entries under '${root}'.`;
}
