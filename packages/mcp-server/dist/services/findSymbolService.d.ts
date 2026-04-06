import { type FindSymbolData, type FindSymbolInput } from "../contracts/public/code.js";
import { type ResponseEnvelope } from "../contracts/public/common.js";
import type { EngineClient } from "../engine/rustEngineClient.js";
export interface FindSymbolService {
    execute(input: FindSymbolInput): Promise<ResponseEnvelope<FindSymbolData>>;
    validateAndExecute(payload: Record<string, unknown>): Promise<ResponseEnvelope<FindSymbolData>>;
}
export declare function createFindSymbolService(options: {
    workspaceRoot: string;
    engineClient: EngineClient;
}): FindSymbolService;
