import {
  type PublicFramework,
  type PublicLanguage,
  type TraceCallersData,
  type TraceCallersInput,
  normalizeTraceCallersInput,
  type ValidationIssue,
} from "../contracts/public/code.js";
import {
  createResponseMeta,
  type ResponseEnvelope,
} from "../contracts/public/common.js";
import {
  nextRequestId,
  type AnalyzerLanguage,
  type TraceCallersEngineResult,
} from "../engine/protocol.js";
import type { EngineClient } from "../engine/rustEngineClient.js";

const TOOL_NAME = "code.trace_callers";
const DEFAULT_MAX_DEPTH = 3;
const MAX_MAX_DEPTH = 8;

export interface TraceCallersService {
  execute(input: TraceCallersInput): Promise<ResponseEnvelope<TraceCallersData>>;
  validateAndExecute(
    payload: Record<string, unknown>,
  ): Promise<ResponseEnvelope<TraceCallersData>>;
}

export function createTraceCallersService(options: {
  workspaceRoot: string;
  engineClient: EngineClient;
}): TraceCallersService {
  return {
    async execute(input) {
      let response;
      try {
        response = await options.engineClient.request<TraceCallersEngineResult>({
          id: nextRequestId(),
          capability: "workspace.trace_callers",
          workspaceRoot: options.workspaceRoot,
          payload: {
            path: input.path,
            symbol: input.symbol,
            analyzerLanguage: resolveAnalyzerLanguage(input.language, input.framework),
            publicLanguageFilter: resolveEffectiveLanguage(input.language, input.framework),
            recursive: input.recursive,
            maxDepth: input.recursive ? resolveMaxDepth(input.max_depth) : null,
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
      const normalized = normalizeTraceCallersInput(payload);
      if (!normalized.ok) {
        return buildValidationErrorResponse(normalized.issues);
      }
      return this.execute(normalized.value);
    },
  };
}

function buildSuccessResponse(
  input: TraceCallersInput,
  result: TraceCallersEngineResult,
): ResponseEnvelope<TraceCallersData> {
  const effectiveLanguage = resolveEffectiveLanguage(input.language, input.framework);
  const count = result.totalMatched;

  return {
    tool: TOOL_NAME,
    status: result.truncated ? "partial" : "ok",
    summary: buildSummary(input.symbol, input.path, count, input.recursive),
    data: {
      target: {
        path: input.path,
        symbol: input.symbol,
        language: inferLanguageFromPath(input.path),
      },
      count,
      returnedCount: result.items.length,
      items: result.items,
      recursive: result.recursive,
    },
    errors: result.truncated
      ? [
          {
            code: "RESULT_TRUNCATED",
            message: "Recursive reverse-trace payload exceeded the response safety caps.",
            retryable: false,
            suggestion: "Retry with a lower max_depth or disable recursive mode.",
            details: {
              maxDepth: input.recursive ? resolveMaxDepth(input.max_depth) : null,
            },
          },
        ]
      : [],
    meta: createResponseMeta({
      query: { ...input },
      resolvedPath: result.resolvedPath,
      truncated: result.truncated,
      counts: {
        returnedCount: result.items.length,
        totalMatched: count,
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
): ResponseEnvelope<TraceCallersData> {
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

function buildMappedErrorResponse(
  input: TraceCallersInput,
  code: string,
  message: string,
  details: Record<string, unknown>,
  retryable: boolean,
): ResponseEnvelope<TraceCallersData> {
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
      summary: "Caller trace failed.",
      data: emptyData(input.path, input.symbol),
      errors: [
        {
          code: "BACKEND_EXECUTION_FAILED",
          message,
          retryable,
          suggestion: "Verify the engine supports workspace.trace_callers and retry.",
          details,
        },
      ],
      meta: createResponseMeta({ query }),
    };
  }

  return {
    tool: TOOL_NAME,
    status: "error",
    summary: "Caller trace failed.",
    data: emptyData(input.path, input.symbol),
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
  input: TraceCallersInput,
  error: unknown,
): ResponseEnvelope<TraceCallersData> {
  return buildMappedErrorResponse(
    input,
    "BACKEND_EXECUTION_FAILED",
    error instanceof Error ? error.message : String(error),
    {},
    false,
  );
}

function emptyData(path = "", symbol = ""): TraceCallersData {
  return {
    target: {
      path,
      symbol,
      language: inferLanguageFromPath(path),
    },
    count: 0,
    returnedCount: 0,
    items: [],
    recursive: null,
  };
}

function buildSummary(
  symbol: string,
  path: string,
  count: number,
  recursive: boolean,
): string {
  if (count === 0) {
    return `Trace completed for incoming callers of '${symbol}' from '${path}' with no callers found.`;
  }
  if (recursive) {
    if (count === 1) {
      return `Found 1 incoming caller for '${symbol}' from '${path}' with recursive reverse trace.`;
    }
    return `Found ${count} incoming callers for '${symbol}' from '${path}' with recursive reverse trace.`;
  }
  if (count === 1) {
    return `Found 1 incoming caller for '${symbol}' from '${path}'.`;
  }
  return `Found ${count} incoming callers for '${symbol}' from '${path}'.`;
}

function resolveMaxDepth(maxDepth: number | null | undefined): number {
  return Math.min(maxDepth ?? DEFAULT_MAX_DEPTH, MAX_MAX_DEPTH);
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
  return "typescript";
}

function inferLanguageFromPath(path: string): PublicLanguage | null {
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
