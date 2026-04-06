import { type TraceFlowData, type TraceFlowInput } from "../contracts/public/code.js";
import { type ResponseEnvelope } from "../contracts/public/common.js";
import type { EngineClient } from "../engine/rustEngineClient.js";
export interface TraceFlowService {
    execute(input: TraceFlowInput): Promise<ResponseEnvelope<TraceFlowData>>;
    validateAndExecute(payload: Record<string, unknown>): Promise<ResponseEnvelope<TraceFlowData>>;
}
export declare function createTraceFlowService(options: {
    workspaceRoot: string;
    engineClient: EngineClient;
}): TraceFlowService;
