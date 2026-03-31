import { type ListEndpointsData, type ListEndpointsInput } from "../contracts/public/code.ts";
import { type ResponseEnvelope } from "../contracts/public/common.ts";
import type { EngineClient } from "../engine/rustEngineClient.ts";
export interface ListEndpointsService {
    execute(input: ListEndpointsInput): Promise<ResponseEnvelope<ListEndpointsData>>;
    validateAndExecute(payload: Record<string, unknown>): Promise<ResponseEnvelope<ListEndpointsData>>;
}
export declare function createListEndpointsService(options: {
    workspaceRoot: string;
    engineClient: EngineClient;
}): ListEndpointsService;
