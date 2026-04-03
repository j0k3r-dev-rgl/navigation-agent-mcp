import {
  type TraceFlowData,
  type TraceFlowInput,
  normalizeTraceFlowInput,
  type ValidationIssue,
} from "../contracts/public/code.js";
import {
  createResponseMeta,
  type ResponseEnvelope,
} from "../contracts/public/common.js";
import {
  nextRequestId,
  type TraceFlowEngineResult,
} from "../engine/protocol.js";
import type { EngineClient } from "../engine/rustEngineClient.js";
import {
  inferLanguageFromPath,
  resolveAnalyzerLanguage,
  resolveEffectiveLanguage,
} from "./languageResolution.js";

const TOOL_NAME = "code.trace_flow";

export interface TraceFlowService {
  execute(input: TraceFlowInput): Promise<ResponseEnvelope<TraceFlowData>>;
  validateAndExecute(
    payload: Record<string, unknown>,
  ): Promise<ResponseEnvelope<TraceFlowData>>;
}

export function createTraceFlowService(options: {
  workspaceRoot: string;
  engineClient: EngineClient;
}): TraceFlowService {
  return {
    async execute(input) {
      let response;
      try {
        response = await options.engineClient.request<TraceFlowEngineResult>({
          id: nextRequestId(),
          capability: "workspace.trace_flow",
          workspaceRoot: options.workspaceRoot,
          payload: {
            path: input.path,
            symbol: input.symbol,
            analyzerLanguage: resolveAnalyzerLanguage(input.language, input.framework),
            publicLanguageFilter: resolveEffectiveLanguage(input.language, input.framework),
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
      const normalized = normalizeTraceFlowInput(payload);
      if (!normalized.ok) {
        return buildValidationErrorResponse(normalized.issues);
      }
      return this.execute(normalized.value);
    },
  };
}

function buildSuccessResponse(
  input: TraceFlowInput,
  result: TraceFlowEngineResult,
): ResponseEnvelope<TraceFlowData> {
  const entrypointPath = result.resolvedPath ?? input.path;
  const effectiveLanguage = resolveEffectiveLanguage(input.language, input.framework);
  const fileCount = result.items.length;
  const calleeCount = result.callees?.length ?? 0;

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
      fileCount,
      items: result.items.map((item) => ({
        path: item.path,
        language: item.language,
      })),
      callees: (result.callees ?? []).map((callee) => ({
        path: callee.path,
        line: callee.line,
        endLine: callee.endLine,
        column: callee.column,
        callee: callee.callee,
        calleeSymbol: callee.calleeSymbol,
        relation: callee.relation,
        language: callee.language,
        snippet: callee.snippet,
        depth: callee.depth,
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

function buildValidationErrorResponse(
  issues: ValidationIssue[],
): ResponseEnvelope<TraceFlowData> {
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
  input: TraceFlowInput,
  code: string,
  message: string,
  details: Record<string, unknown>,
  retryable: boolean,
): ResponseEnvelope<TraceFlowData> {
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

function buildEngineFailureResponse(
  input: TraceFlowInput,
  error: unknown,
): ResponseEnvelope<TraceFlowData> {
  return buildMappedErrorResponse(
    input,
    "BACKEND_EXECUTION_FAILED",
    error instanceof Error ? error.message : String(error),
    {},
    false,
  );
}

function emptyData(path = "", symbol = ""): TraceFlowData {
  return {
    entrypoint: {
      path,
      symbol,
      language: inferLanguageFromPath(path),
    },
    fileCount: 0,
    items: [],
    callees: [],
  };
}

function buildSummary(symbol: string, path: string, calleeCount: number): string {
  if (calleeCount === 0) {
    return `Trace completed for '${symbol}' from '${path}' with no callees found.`;
  }
  if (calleeCount === 1) {
    return `Traced 1 callee for '${symbol}' from '${path}'.`;
  }
  return `Traced ${calleeCount} callees for '${symbol}' from '${path}'.`;
}
