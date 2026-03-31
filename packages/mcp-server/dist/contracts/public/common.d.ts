export declare const RESPONSE_STATUSES: readonly ["ok", "partial", "error"];
export type ResponseStatus = (typeof RESPONSE_STATUSES)[number];
export declare const ERROR_CODES: readonly ["INVALID_INPUT", "PATH_OUTSIDE_WORKSPACE", "FILE_NOT_FOUND", "SYMBOL_NOT_FOUND", "UNSUPPORTED_FILE", "BACKEND_SCRIPT_NOT_FOUND", "BACKEND_DEPENDENCY_NOT_FOUND", "BACKEND_EXECUTION_FAILED", "BACKEND_INVALID_RESPONSE", "RESULT_TRUNCATED"];
export type ErrorCode = (typeof ERROR_CODES)[number];
export interface ErrorItem {
    code: ErrorCode | string;
    message: string;
    retryable: boolean;
    suggestion?: string | null;
    target?: string | null;
    details: Record<string, unknown>;
}
export interface ResponseMeta {
    query: Record<string, unknown>;
    resolvedPath: string | null;
    truncated: boolean;
    counts: Record<string, number | null>;
    detection: Record<string, string | null>;
}
export interface ResponseEnvelope<TData> {
    tool: string;
    status: ResponseStatus;
    summary: string;
    data: TData;
    errors: ErrorItem[];
    meta: ResponseMeta;
}
export declare function createResponseMeta(overrides?: Partial<ResponseMeta>): ResponseMeta;
