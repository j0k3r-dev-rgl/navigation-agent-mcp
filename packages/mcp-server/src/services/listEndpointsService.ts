import {
	type EndpointDefinition,
	type ListEndpointsCounts,
	type ListEndpointsData,
	type ListEndpointsInput,
	normalizeListEndpointsInput,
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
	type ListEndpointsEngineResult,
} from "../engine/protocol.ts";
import type { EngineClient } from "../engine/rustEngineClient.ts";

const TOOL_NAME = "code.list_endpoints";

export interface ListEndpointsService {
	execute(input: ListEndpointsInput): Promise<ResponseEnvelope<ListEndpointsData>>;
	validateAndExecute(
		payload: Record<string, unknown>,
	): Promise<ResponseEnvelope<ListEndpointsData>>;
}

export function createListEndpointsService(options: {
	workspaceRoot: string;
	engineClient: EngineClient;
}): ListEndpointsService {
	return {
		async execute(input) {
			let response;
			try {
				response = await options.engineClient.request<ListEndpointsEngineResult>({
					id: nextRequestId(),
					capability: "workspace.list_endpoints",
					workspaceRoot: options.workspaceRoot,
					payload: {
						path: input.path ?? null,
						analyzerLanguage: resolveAnalyzerLanguage(input.language, input.framework),
						publicLanguageFilter: resolveEffectiveLanguage(input.language, input.framework),
						publicFrameworkFilter: input.framework ?? null,
						kind: input.kind,
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
			const normalized = normalizeListEndpointsInput(payload);
			if (!normalized.ok) {
				return buildValidationErrorResponse(normalized.issues);
			}
			return this.execute(normalized.value);
		},
	};
}

function buildSuccessResponse(
	input: ListEndpointsInput,
	result: ListEndpointsEngineResult,
): ResponseEnvelope<ListEndpointsData> {
	const effectiveLanguage = resolveEffectiveLanguage(input.language, input.framework);
	const totalCount = result.totalMatched;
	const returnedCount = result.items.length;

	return {
		tool: TOOL_NAME,
		status: result.truncated ? "partial" : "ok",
		summary: buildSummary(totalCount, result.truncated, input.kind),
		data: {
			totalCount,
			returnedCount,
			counts: mapCounts(result.counts),
			items: result.items.map((item) => ({
				name: item.name,
				kind: item.kind as EndpointDefinition["kind"],
				path: item.path ?? null,
				file: item.file,
				line: item.line,
				language: item.language ?? null,
				framework: item.framework ?? null,
			})),
		},
		errors: result.truncated
			? [
					{
						code: "RESULT_TRUNCATED",
						message: `Result set exceeded the requested limit of ${input.limit} items.`,
						retryable: false,
						suggestion: "Increase limit or narrow the path/language/framework filter.",
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

function mapCounts(counts: ListEndpointsEngineResult["counts"]): ListEndpointsCounts {
	return {
		byKind: counts.byKind,
		byLanguage: counts.byLanguage,
		byFramework: counts.byFramework,
	};
}

function buildValidationErrorResponse(
	issues: ValidationIssue[],
): ResponseEnvelope<ListEndpointsData> {
	return {
		tool: TOOL_NAME,
		status: "error",
		summary: "Request validation failed.",
		data: { totalCount: 0, returnedCount: 0, counts: { byKind: {}, byLanguage: {}, byFramework: {} }, items: [] },
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
	input: ListEndpointsInput,
	code: string,
	message: string,
	details: Record<string, unknown>,
	retryable: boolean,
): ResponseEnvelope<ListEndpointsData> {
	const query = { ...input };

	if (code === "FILE_NOT_FOUND") {
		return {
			tool: TOOL_NAME,
			status: "error",
			summary: "Path not found.",
			data: { totalCount: 0, returnedCount: 0, counts: { byKind: {}, byLanguage: {}, byFramework: {} }, items: [] },
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
			data: { totalCount: 0, returnedCount: 0, counts: { byKind: {}, byLanguage: {}, byFramework: {} }, items: [] },
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
			summary: "Endpoint analysis failed.",
			data: { totalCount: 0, returnedCount: 0, counts: { byKind: {}, byLanguage: {}, byFramework: {} }, items: [] },
			errors: [
				{
					code: "BACKEND_EXECUTION_FAILED",
					message,
					retryable,
					suggestion: "Verify the engine supports workspace.list_endpoints and retry.",
					details,
				},
			],
			meta: createResponseMeta({ query }),
		};
	}

	return {
		tool: TOOL_NAME,
		status: "error",
		summary: "Endpoint analysis failed.",
		data: { totalCount: 0, returnedCount: 0, counts: { byKind: {}, byLanguage: {}, byFramework: {} }, items: [] },
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
	input: ListEndpointsInput,
	error: unknown,
): ResponseEnvelope<ListEndpointsData> {
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

function buildSummary(count: number, truncated: boolean, kind: string): string {
	const kindLabel = kind === "any" ? "endpoints" : `${kind} endpoints`;
	if (count === 0) {
		return `No${kindLabel} found.`;
	}
	if (truncated) {
		return `Found ${count} ${kindLabel} and returned a truncated subset.`;
	}
	if (count === 1) {
		return `Found 1 ${kindLabel.replace(/s$/, "")}.`;
	}
	return `Found ${count} ${kindLabel}.`;
}