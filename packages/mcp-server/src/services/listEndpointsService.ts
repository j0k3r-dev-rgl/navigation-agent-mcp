import {
	type EndpointDefinition,
	type ListEndpointsCounts,
	type ListEndpointsData,
	type ListEndpointsInput,
	normalizeListEndpointsInput,
	type ValidationIssue,
} from "../contracts/public/code.js";
import {
	createResponseMeta,
	type ResponseEnvelope,
} from "../contracts/public/common.js";
import {
	nextRequestId,
	type ListEndpointsEngineResult,
} from "../engine/protocol.js";
import type { EngineClient } from "../engine/rustEngineClient.js";
import { resolveAnalyzerLanguage, resolveEffectiveLanguage } from "./languageResolution.js";

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
					analyzerLanguage: resolveAnalyzerLanguage(input.language, input.framework, input.path),
					publicLanguageFilter: resolveEffectiveLanguage(input.language, input.framework, input.path),
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
	const effectiveLanguage = resolveEffectiveLanguage(input.language, input.framework, input.path);
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
						suggestion:
							"Increase limit or narrow the path/language/framework filter. Use this tool to inventory likely public entrypoints, then switch to find_symbol or trace tools for deeper analysis.",
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
				suggestion:
					"Correct the invalid fields and try again. Use framework and kind filters to describe the entrypoint surface you want to inventory.",
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
					suggestion:
						"Provide an existing file or directory path inside the workspace root, or omit the path filter to inventory endpoints across the whole workspace.",
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
					suggestion:
						"Use a path inside the workspace root or omit the path filter. This tool only inventories framework-detectable entrypoints inside the current workspace.",
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
					suggestion:
						"Verify the engine supports workspace.list_endpoints and retry. If you need a specific handler's logic or impact, continue with find_symbol, trace_callers, or trace_flow instead.",
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

function buildSummary(count: number, truncated: boolean, kind: string): string {
	const kindLabel = kind === "any" ? "endpoints" : `${kind} endpoints`;
	if (count === 0) {
		return `No ${kindLabel} found. This tool only reports framework-detectable public entrypoints in the current workspace.`;
	}
	if (truncated) {
		return `Found ${count} ${kindLabel} and returned a truncated subset. Use the results to locate likely entrypoints, then switch to find_symbol or trace tools for deeper analysis.`;
	}
	if (count === 1) {
		return `Found 1 ${kindLabel.replace(/s$/, "")}. Use it as a likely public entrypoint before reading implementation files.`;
	}
	return `Found ${count} ${kindLabel}. Use this inventory to map the exposed route surface before deeper symbol or flow analysis.`;
}
