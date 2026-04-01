import { type TraceFlowData, type TraceFlowInput } from "../contracts/public/code.ts";
import { type ResponseEnvelope } from "../contracts/public/common.ts";
import type { EngineClient } from "../engine/rustEngineClient.ts";
export interface TraceFlowService {
    execute(input: TraceFlowInput): Promise<ResponseEnvelope<TraceFlowData>>;
    validateAndExecute(payload: Record<string, unknown>): Promise<ResponseEnvelope<TraceFlowData>>;
}
export declare function createTraceFlowService(options: {
    workspaceRoot: string;
    engineClient: EngineClient;
}): TraceFlowService;
