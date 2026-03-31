import { type SearchTextData, type SearchTextInput } from "../contracts/public/code.ts";
import { type ResponseEnvelope } from "../contracts/public/common.ts";
import type { EngineClient } from "../engine/rustEngineClient.ts";
export interface SearchTextService {
    execute(input: SearchTextInput): Promise<ResponseEnvelope<SearchTextData>>;
    validateAndExecute(payload: Record<string, unknown>): Promise<ResponseEnvelope<SearchTextData>>;
}
export declare function createSearchTextService(options: {
    workspaceRoot: string;
    engineClient: EngineClient;
}): SearchTextService;
