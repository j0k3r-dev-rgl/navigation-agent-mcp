import { type CodeToolName } from "../contracts/public/code.ts";
import type { ResponseEnvelope } from "../contracts/public/common.ts";
import * as z from "zod/v4";
export interface RegisteredCodeTool {
    name: CodeToolName;
    title: string;
    description: string;
    inputSchema: Record<string, unknown>;
    sdkInputSchema: Record<string, z.ZodType>;
    execute(payload: Record<string, unknown>): Promise<ResponseEnvelope<unknown>>;
}
export interface RegisterCodeToolsOptions {
    inspectTreeHandler?: (payload: Record<string, unknown>) => Promise<ResponseEnvelope<unknown>>;
    findSymbolHandler?: (payload: Record<string, unknown>) => Promise<ResponseEnvelope<unknown>>;
    listEndpointsHandler?: (payload: Record<string, unknown>) => Promise<ResponseEnvelope<unknown>>;
    searchTextHandler?: (payload: Record<string, unknown>) => Promise<ResponseEnvelope<unknown>>;
    traceCallersHandler?: (payload: Record<string, unknown>) => Promise<ResponseEnvelope<unknown>>;
    traceFlowHandler?: (payload: Record<string, unknown>) => Promise<ResponseEnvelope<unknown>>;
}
export declare function registerCodeTools(options: RegisterCodeToolsOptions): RegisteredCodeTool[];
