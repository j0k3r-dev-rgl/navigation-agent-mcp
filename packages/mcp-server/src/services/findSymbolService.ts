import {
  type FindSymbolData,
  type FindSymbolInput,
  normalizeFindSymbolInput,
  type ValidationIssue,
} from "../contracts/public/code.js";
import {
  createResponseMeta,
  type ResponseEnvelope,
} from "../contracts/public/common.js";
import {
  nextRequestId,
  type FindSymbolEngineResult,
} from "../engine/protocol.js";
import type { EngineClient } from "../engine/rustEngineClient.js";
import { resolveAnalyzerLanguage, resolveEffectiveLanguage } from "./languageResolution.js";

const TOOL_NAME = "code.find_symbol";

export interface FindSymbolService {
  execute(input: FindSymbolInput): Promise<ResponseEnvelope<FindSymbolData>>;
  validateAndExecute(
    payload: Record<string, unknown>,
  ): Promise<ResponseEnvelope<FindSymbolData>>;
}

export function createFindSymbolService(options: {
  workspaceRoot: string;
  engineClient: EngineClient;
}): FindSymbolService {
  return {
    async execute(input) {
      let response;
      try {
        response = await options.engineClient.request<FindSymbolEngineResult>({
          id: nextRequestId(),
          capability: "workspace.find_symbol",
          workspaceRoot: options.workspaceRoot,
          payload: {
            symbol: input.symbol,
            path: input.path ?? null,
            analyzerLanguage: resolveAnalyzerLanguage(input.language, input.framework, input.path),
            publicLanguageFilter: resolveEffectiveLanguage(input.language, input.framework, input.path),
            kind: input.kind,
            matchMode: input.match,
            limit: input.limit,
          },
        });
      } catch (error) {
        return buildEngineFailureResponse(input, error);
      }

      if (!response.ok) {
        return buildMappedErrorResponse(
          input,
          response.error.code,
          response.error.message,
          response.error.details,
          response.error.retryable,
        );
      }

      return buildSuccessResponse(input, response.result);
    },
    async validateAndExecute(payload) {
      const normalized = normalizeFindSymbolInput(payload);
      if (!normalized.ok) {
        return buildValidationErrorResponse(normalized.issues);
      }
      return this.execute(normalized.value);
    },
  };
}

function buildSuccessResponse(
  input: FindSymbolInput,
  result: FindSymbolEngineResult,
): ResponseEnvelope<FindSymbolData> {
  const effectiveLanguage = resolveEffectiveLanguage(input.language, input.framework, input.path);
  const count = result.totalMatched;
  const returnedCount = result.items.length;

  return {
    tool: TOOL_NAME,
    status: result.truncated ? "partial" : "ok",
    summary: buildSummary(input.symbol, count, result.truncated),
    data: {
      count,
      returnedCount,
      totalMatched: result.totalMatched,
      items: result.items.map((item) => ({
        symbol: item.symbol,
        kind: item.kind,
        path: item.path,
        line: item.line,
        lineEnd: item.lineEnd,
        language: item.language,
      })),
    },
    errors: result.truncated
      ? [
          {
            code: "RESULT_TRUNCATED",
            message: `Result set exceeded the requested limit of ${input.limit} items.`,
            retryable: false,
            suggestion:
              "Increase limit or narrow the path/language filter. Once you identify the right definition, pass its path to code.trace_callers or code.trace_flow instead of reading files broadly.",
            details: {
              returned: returnedCount,
              total: result.totalMatched,
              limit: input.limit,
            },
          },
        ]
      : [],
    meta: createResponseMeta({
      query: { ...input },
      resolvedPath: result.resolvedPath,
      truncated: result.truncated,
      counts: {
        returnedCount,
        totalMatched: result.totalMatched,
      },
      detection: {
        effectiveLanguage,
        framework: input.framework ?? null,
      },
    }),
  };
}

function buildValidationErrorResponse(
  issues: ValidationIssue[],
): ResponseEnvelope<FindSymbolData> {
  return {
    tool: TOOL_NAME,
    status: "error",
    summary: "Request validation failed.",
    data: { count: 0, returnedCount: 0, totalMatched: 0, items: [] },
    errors: [
      {
        code: "INVALID_INPUT",
        message: "One or more input fields are invalid.",
        retryable: false,
        suggestion:
          "Correct the invalid fields and try again. For tracing workflows, provide at least the symbol name and add path/language filters only when they help narrow the workspace search.",
        details: { issues },
      },
    ],
    meta: createResponseMeta({ query: {} }),
  };
}

function buildMappedErrorResponse(
  input: FindSymbolInput,
  code: string,
  message: string,
  details: Record<string, unknown>,
  retryable: boolean,
): ResponseEnvelope<FindSymbolData> {
  const query = { ...input };

  if (code === "FILE_NOT_FOUND") {
    return {
      tool: TOOL_NAME,
      status: "error",
      summary: "Path not found.",
      data: { count: 0, returnedCount: 0, totalMatched: 0, items: [] },
      errors: [
        {
          code,
          message,
          retryable,
          suggestion:
            "Provide an existing file or directory path inside the workspace root, or omit the path filter and search by symbol name first.",
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
      data: { count: 0, returnedCount: 0, totalMatched: 0, items: [] },
      errors: [
        {
          code,
          message,
          retryable,
          suggestion:
            "Use a path inside the workspace root or omit the path filter. This tool only resolves definitions inside the current workspace.",
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
      summary: "Symbol analysis failed.",
      data: { count: 0, returnedCount: 0, totalMatched: 0, items: [] },
      errors: [
        {
          code: "BACKEND_EXECUTION_FAILED",
          message,
          retryable,
          suggestion:
            "Verify the engine supports workspace.find_symbol and retry. If successful, use the returned definition path as the handoff into code.trace_callers or code.trace_flow.",
          details,
        },
      ],
      meta: createResponseMeta({ query }),
    };
  }

  return {
    tool: TOOL_NAME,
    status: "error",
    summary: "Symbol analysis failed.",
    data: { count: 0, returnedCount: 0, totalMatched: 0, items: [] },
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

function buildEngineFailureResponse(
  input: FindSymbolInput,
  error: unknown,
): ResponseEnvelope<FindSymbolData> {
  return buildMappedErrorResponse(
    input,
    "BACKEND_EXECUTION_FAILED",
    error instanceof Error ? error.message : String(error),
    {},
    false,
  );
}

function buildSummary(symbol: string, count: number, truncated: boolean): string {
  if (count === 0) {
    return `No symbol definitions found for '${symbol}'. If you only need textual mentions, try code.search_text; if you expected a definition, narrow the workspace path, language, or symbol spelling.`;
  }
  if (truncated) {
    return `Found ${count} symbol definitions for '${symbol}' and returned a truncated subset. Choose the correct defining path, then hand it to code.trace_callers or code.trace_flow before reading files.`;
  }
  if (count === 1) {
    return `Found 1 symbol definition for '${symbol}'. Use its path as the next step for code.trace_callers or code.trace_flow.`;
  }
  return `Found ${count} symbol definitions for '${symbol}'. Select the right definition, then use its path for code.trace_callers or code.trace_flow.`;
}
