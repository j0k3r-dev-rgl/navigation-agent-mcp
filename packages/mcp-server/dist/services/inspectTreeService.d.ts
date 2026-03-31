import { type InspectTreeData, type InspectTreeInput } from "../contracts/public/code.ts";
import { type ResponseEnvelope } from "../contracts/public/common.ts";
import type { EngineClient } from "../engine/rustEngineClient.ts";
export interface InspectTreeService {
    execute(input: InspectTreeInput): Promise<ResponseEnvelope<InspectTreeData>>;
    validateAndExecute(payload: Record<string, unknown>): Promise<ResponseEnvelope<InspectTreeData>>;
}
export declare function createInspectTreeService(options: {
    workspaceRoot: string;
    engineClient: EngineClient;
}): InspectTreeService;
