import { type TraceCallersData, type TraceCallersInput } from "../contracts/public/code.js";
import { type ResponseEnvelope } from "../contracts/public/common.js";
import type { EngineClient } from "../engine/rustEngineClient.js";
export interface TraceCallersService {
    execute(input: TraceCallersInput): Promise<ResponseEnvelope<TraceCallersData>>;
    validateAndExecute(payload: Record<string, unknown>): Promise<ResponseEnvelope<TraceCallersData>>;
}
export declare function createTraceCallersService(options: {
    workspaceRoot: string;
    engineClient: EngineClient;
}): TraceCallersService;
