import {
  type FindSymbolData,
  type FindSymbolInput,
  normalizeFindSymbolInput,
  type PublicFramework,
  type PublicLanguage,
  type ValidationIssue,
} from "../contracts/public/code.ts";
import {
  createResponseMeta,
  type ResponseEnvelope,
} from "../contracts/public/common.ts";
import {
  nextRequestId,
  type AnalyzerLanguage,
  type FindSymbolEngineResult,
} from "../engine/protocol.ts";
import type { EngineClient } from "../engine/rustEngineClient.ts";

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
            analyzerLanguage: resolveAnalyzerLanguage(input.language, input.framework),
            publicLanguageFilter: resolveEffectiveLanguage(input.language, input.framework),
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
  const effectiveLanguage = resolveEffectiveLanguage(input.language, input.framework);
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
            suggestion: "Increase limit or narrow the path/language filter.",
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
        suggestion: "Correct the invalid fields and try again.",
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
          suggestion: "Provide an existing file or directory path inside the workspace root.",
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
          suggestion: "Use a path inside the workspace root or omit the path filter.",
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
          suggestion: "Verify the engine supports workspace.find_symbol and retry.",
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

function resolveEffectiveLanguage(
  language: PublicLanguage | null | undefined,
  framework: PublicFramework | null | undefined,
): PublicLanguage | null {
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

function resolveAnalyzerLanguage(
  language: PublicLanguage | null | undefined,
  framework: PublicFramework | null | undefined,
): AnalyzerLanguage {
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
  if (effective === "typescript" || effective === "javascript") {
    return "typescript";
  }
  return "auto";
}

function buildSummary(symbol: string, count: number, truncated: boolean): string {
  if (count === 0) {
    return `No symbol definitions found for '${symbol}'.`;
  }
  if (truncated) {
    return `Found ${count} symbol definitions for '${symbol}' and returned a truncated subset.`;
  }
  if (count === 1) {
    return `Found 1 symbol definition for '${symbol}'.`;
  }
  return `Found ${count} symbol definitions for '${symbol}'.`;
}
