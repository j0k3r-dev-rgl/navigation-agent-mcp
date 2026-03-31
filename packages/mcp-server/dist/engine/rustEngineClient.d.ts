import type { EngineRequest, EngineResponse } from "./protocol.ts";
export interface EngineClient {
    request<TResult = unknown>(request: EngineRequest): Promise<EngineResponse<TResult>>;
    close(): Promise<void>;
}
export interface RustEngineClientOptions {
    command?: readonly string[];
    cwd?: string;
    env?: NodeJS.ProcessEnv;
}
export declare class RustEngineClient implements EngineClient {
    #private;
    constructor(options?: RustEngineClientOptions);
    request<TResult = unknown>(request: EngineRequest): Promise<EngineResponse<TResult>>;
    close(): Promise<void>;
}
