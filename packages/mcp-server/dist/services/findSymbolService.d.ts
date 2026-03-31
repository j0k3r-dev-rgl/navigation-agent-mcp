import { type FindSymbolData, type FindSymbolInput } from "../contracts/public/code.ts";
import { type ResponseEnvelope } from "../contracts/public/common.ts";
import type { EngineClient } from "../engine/rustEngineClient.ts";
export interface FindSymbolService {
    execute(input: FindSymbolInput): Promise<ResponseEnvelope<FindSymbolData>>;
    validateAndExecute(payload: Record<string, unknown>): Promise<ResponseEnvelope<FindSymbolData>>;
}
export declare function createFindSymbolService(options: {
    workspaceRoot: string;
    engineClient: EngineClient;
}): FindSymbolService;
