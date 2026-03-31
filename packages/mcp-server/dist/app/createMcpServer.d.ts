import { type EngineClient } from "../engine/rustEngineClient.ts";
import { type RegisteredCodeTool } from "../tools/registerCodeTools.ts";
export interface CreateMcpServerOptions {
    workspaceRoot: string;
    engineClient?: EngineClient;
}
export interface McpServerPlan {
    name: "navigation-agent-mcp";
    version: "0.1.0";
    workspaceRoot: string;
    tools: RegisteredCodeTool[];
    listTools(): RegisteredCodeTool[];
    callTool(name: string, payload: Record<string, unknown>): Promise<unknown>;
    serveStdio(): Promise<void>;
    serveStdioLegacy(): Promise<void>;
    close(): Promise<void>;
}
export declare function createMcpServer(options: CreateMcpServerOptions): McpServerPlan;
