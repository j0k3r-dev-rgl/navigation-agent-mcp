import { type TraceSymbolData, type TraceSymbolInput } from "../contracts/public/code.ts";
import { type ResponseEnvelope } from "../contracts/public/common.ts";
import type { EngineClient } from "../engine/rustEngineClient.ts";
export interface TraceSymbolService {
    execute(input: TraceSymbolInput): Promise<ResponseEnvelope<TraceSymbolData>>;
    validateAndExecute(payload: Record<string, unknown>): Promise<ResponseEnvelope<TraceSymbolData>>;
}
export declare function createTraceSymbolService(options: {
    workspaceRoot: string;
    engineClient: EngineClient;
}): TraceSymbolService;
