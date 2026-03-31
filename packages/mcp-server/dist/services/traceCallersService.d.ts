import { type TraceCallersData, type TraceCallersInput } from "../contracts/public/code.ts";
import { type ResponseEnvelope } from "../contracts/public/common.ts";
import type { EngineClient } from "../engine/rustEngineClient.ts";
export interface TraceCallersService {
    execute(input: TraceCallersInput): Promise<ResponseEnvelope<TraceCallersData>>;
    validateAndExecute(payload: Record<string, unknown>): Promise<ResponseEnvelope<TraceCallersData>>;
}
export declare function createTraceCallersService(options: {
    workspaceRoot: string;
    engineClient: EngineClient;
}): TraceCallersService;
