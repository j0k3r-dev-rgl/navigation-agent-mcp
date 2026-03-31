export const RESPONSE_STATUSES = ["ok", "partial", "error"];
export const ERROR_CODES = [
    "INVALID_INPUT",
    "PATH_OUTSIDE_WORKSPACE",
    "FILE_NOT_FOUND",
    "SYMBOL_NOT_FOUND",
    "UNSUPPORTED_FILE",
    "BACKEND_SCRIPT_NOT_FOUND",
    "BACKEND_DEPENDENCY_NOT_FOUND",
    "BACKEND_EXECUTION_FAILED",
    "BACKEND_INVALID_RESPONSE",
    "RESULT_TRUNCATED",
];
export function createResponseMeta(overrides = {}) {
    return {
        query: {},
        resolvedPath: null,
        truncated: false,
        counts: {},
        detection: {},
        ...overrides,
    };
}
