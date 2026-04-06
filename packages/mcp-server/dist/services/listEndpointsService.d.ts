import { type ListEndpointsData, type ListEndpointsInput } from "../contracts/public/code.js";
import { type ResponseEnvelope } from "../contracts/public/common.js";
import type { EngineClient } from "../engine/rustEngineClient.js";
export interface ListEndpointsService {
    execute(input: ListEndpointsInput): Promise<ResponseEnvelope<ListEndpointsData>>;
    validateAndExecute(payload: Record<string, unknown>): Promise<ResponseEnvelope<ListEndpointsData>>;
}
export declare function createListEndpointsService(options: {
    workspaceRoot: string;
    engineClient: EngineClient;
}): ListEndpointsService;
