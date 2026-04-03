import {
  type SearchTextData,
  type SearchTextInput,
  normalizeSearchTextInput,
  type ValidationIssue,
} from "../contracts/public/code.js";
import {
  createResponseMeta,
  type ResponseEnvelope,
} from "../contracts/public/common.js";
import {
  nextRequestId,
  type SearchTextEngineResult,
} from "../engine/protocol.js";
import type { EngineClient } from "../engine/rustEngineClient.js";
import { resolveEffectiveLanguage } from "./languageResolution.js";

const TOOL_NAME = "code.search_text";

export interface SearchTextService {
  execute(input: SearchTextInput): Promise<ResponseEnvelope<SearchTextData>>;
  validateAndExecute(
    payload: Record<string, unknown>,
  ): Promise<ResponseEnvelope<SearchTextData>>;
}

export function createSearchTextService(options: {
  workspaceRoot: string;
  engineClient: EngineClient;
}): SearchTextService {
  return {
    async execute(input) {
      let response;
      try {
        response = await options.engineClient.request<SearchTextEngineResult>({
          id: nextRequestId(),
          capability: "workspace.search_text",
          workspaceRoot: options.workspaceRoot,
          payload: {
            query: input.query,
            path: input.path ?? null,
            publicLanguageFilter: resolveEffectiveLanguage(input.language, input.framework, input.path),
            include: input.include ?? null,
            regex: input.regex,
            context: input.context,
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
      const normalized = normalizeSearchTextInput(payload);
      if (!normalized.ok) {
        return buildValidationErrorResponse(normalized.issues);
      }
      return this.execute(normalized.value);
    },
  };
}

function buildSuccessResponse(
  input: SearchTextInput,
  result: SearchTextEngineResult,
): ResponseEnvelope<SearchTextData> {
  const returnedFileCount = result.items.length;
  const returnedMatchCount = result.items.reduce(
    (total, item) => total + item.matchCount,
    0,
  );
  const effectiveLanguage = resolveEffectiveLanguage(input.language, input.framework, input.path);

  return {
    tool: TOOL_NAME,
    status: result.truncated ? "partial" : "ok",
    summary: buildSummary(
      input.query,
      result.totalFileCount,
      result.totalMatchCount,
      result.truncated,
    ),
    data: {
      fileCount: result.truncated ? returnedFileCount : result.totalFileCount,
      matchCount: result.truncated ? returnedMatchCount : result.totalMatchCount,
      totalFileCount: result.totalFileCount,
      totalMatchCount: result.totalMatchCount,
      items: result.items.map((item) => ({
        path: item.path,
        language: item.language,
        matchCount: item.matchCount,
        matches: item.matches.map((match) => ({
          line: match.line,
          text: match.text,
          submatches: match.submatches.map((submatch) => ({ ...submatch })),
          before: match.before.map((contextLine) => ({ ...contextLine })),
          after: match.after.map((contextLine) => ({ ...contextLine })),
        })),
      })),
    },
    errors: result.truncated
      ? [
          {
            code: "RESULT_TRUNCATED",
            message: `Result set exceeded the requested limit of ${input.limit} files.`,
            retryable: false,
            suggestion: "Increase limit or narrow the path/include/language filters.",
            details: {
              returnedFiles: returnedFileCount,
              totalFiles: result.totalFileCount,
              returnedMatches: returnedMatchCount,
              totalMatches: result.totalMatchCount,
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
        returnedFileCount,
        totalFileCount: result.totalFileCount,
        returnedMatchCount,
        totalMatchCount: result.totalMatchCount,
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
): ResponseEnvelope<SearchTextData> {
  return {
    tool: TOOL_NAME,
    status: "error",
    summary: "Request validation failed.",
    data: {
      fileCount: 0,
      matchCount: 0,
      totalFileCount: 0,
      totalMatchCount: 0,
      items: [],
    },
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
  input: SearchTextInput,
  code: string,
  message: string,
  details: Record<string, unknown>,
  retryable: boolean,
): ResponseEnvelope<SearchTextData> {
  const query = { ...input };

  if (code === "FILE_NOT_FOUND") {
    return {
      tool: TOOL_NAME,
      status: "error",
      summary: "Path not found.",
      data: emptyData(),
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
      data: emptyData(),
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
      summary: "Text search failed.",
      data: emptyData(),
      errors: [
        {
          code: "BACKEND_EXECUTION_FAILED",
          message,
          retryable,
          suggestion: "Verify the engine supports workspace.search_text and retry.",
          details,
        },
      ],
      meta: createResponseMeta({ query }),
    };
  }

  return {
    tool: TOOL_NAME,
    status: "error",
    summary: "Text search failed.",
    data: emptyData(),
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
  input: SearchTextInput,
  error: unknown,
): ResponseEnvelope<SearchTextData> {
  return buildMappedErrorResponse(
    input,
    "BACKEND_EXECUTION_FAILED",
    error instanceof Error ? error.message : String(error),
    {},
    false,
  );
}

function emptyData(): SearchTextData {
  return {
    fileCount: 0,
    matchCount: 0,
    totalFileCount: 0,
    totalMatchCount: 0,
    items: [],
  };
}

function buildSummary(
  query: string,
  fileCount: number,
  matchCount: number,
  truncated: boolean,
): string {
  if (matchCount === 0) {
    return `No text matches found for '${query}'.`;
  }
  if (truncated) {
    return `Found ${matchCount} text matches across ${fileCount} files for '${query}' and returned a truncated subset.`;
  }
  if (matchCount === 1) {
    return `Found 1 text match in 1 file for '${query}'.`;
  }
  return `Found ${matchCount} text matches across ${fileCount} files for '${query}'.`;
}
