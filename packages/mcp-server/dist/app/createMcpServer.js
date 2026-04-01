import { McpServer as SdkMcpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { RustEngineClient } from "../engine/rustEngineClient.js";
import { createFindSymbolService } from "../services/findSymbolService.js";
import { createInspectTreeService } from "../services/inspectTreeService.js";
import { createListEndpointsService } from "../services/listEndpointsService.js";
import { createSearchTextService } from "../services/searchTextService.js";
import { createTraceCallersService } from "../services/traceCallersService.js";
import { createTraceFlowService } from "../services/traceFlowService.js";
import { registerCodeTools, } from "../tools/registerCodeTools.js";
function toSdkToolResult(result) {
    return {
        content: [
            {
                type: "text",
                text: JSON.stringify(result, null, 2),
            },
        ],
        structuredContent: result,
        isError: result.status === "error",
    };
}
export function createMcpServer(options) {
    const engineClient = options.engineClient ?? new RustEngineClient();
    const inspectTreeService = createInspectTreeService({
        workspaceRoot: options.workspaceRoot,
        engineClient,
    });
    const findSymbolService = createFindSymbolService({
        workspaceRoot: options.workspaceRoot,
        engineClient,
    });
    const listEndpointsService = createListEndpointsService({
        workspaceRoot: options.workspaceRoot,
        engineClient,
    });
    const searchTextService = createSearchTextService({
        workspaceRoot: options.workspaceRoot,
        engineClient,
    });
    const traceFlowService = createTraceFlowService({
        workspaceRoot: options.workspaceRoot,
        engineClient,
    });
    const traceCallersService = createTraceCallersService({
        workspaceRoot: options.workspaceRoot,
        engineClient,
    });
    const tools = registerCodeTools({
        inspectTreeHandler: (payload) => inspectTreeService.validateAndExecute(payload),
        findSymbolHandler: (payload) => findSymbolService.validateAndExecute(payload),
        listEndpointsHandler: (payload) => listEndpointsService.validateAndExecute(payload),
        searchTextHandler: (payload) => searchTextService.validateAndExecute(payload),
        traceCallersHandler: (payload) => traceCallersService.validateAndExecute(payload),
        traceFlowHandler: (payload) => traceFlowService.validateAndExecute(payload),
    });
    const sdkServer = new SdkMcpServer({
        name: "navigation-agent-mcp",
        version: "0.1.0",
    });
    for (const tool of tools) {
        sdkServer.registerTool(tool.name, {
            title: tool.title,
            description: tool.description,
            inputSchema: tool.sdkInputSchema,
        }, async (payload) => toSdkToolResult(await tool.execute(payload)));
    }
    return {
        name: "navigation-agent-mcp",
        version: "0.1.0",
        workspaceRoot: options.workspaceRoot,
        tools,
        listTools() {
            return tools;
        },
        async callTool(name, payload) {
            const tool = tools.find((candidate) => candidate.name === name);
            if (!tool) {
                throw new Error(`Unknown tool '${name}'.`);
            }
            return tool.execute(payload);
        },
        async serveStdio() {
            await sdkServer.connect(new StdioServerTransport());
        },
        async serveStdioLegacy() {
            process.stdin.setEncoding("utf8");
            let buffer = "";
            for await (const chunk of process.stdin) {
                buffer += chunk;
                const lines = buffer.split(/\r?\n/);
                buffer = lines.pop() ?? "";
                for (const line of lines) {
                    const trimmed = line.trim();
                    if (!trimmed) {
                        continue;
                    }
                    let request;
                    try {
                        request = JSON.parse(trimmed);
                    }
                    catch (error) {
                        process.stdout.write(`${JSON.stringify({
                            id: null,
                            ok: false,
                            error: {
                                code: "INVALID_REQUEST",
                                message: error instanceof Error ? error.message : String(error),
                            },
                        })}\n`);
                        continue;
                    }
                    const id = request.id ?? null;
                    if (request.method === "list_tools") {
                        process.stdout.write(`${JSON.stringify({ id, ok: true, result: tools.map(({ execute, sdkInputSchema, ...tool }) => tool) })}\n`);
                        continue;
                    }
                    if (request.method === "call_tool") {
                        const toolName = typeof request.params?.name === "string" ? request.params.name : null;
                        const argumentsPayload = request.params && typeof request.params.arguments === "object" && request.params.arguments
                            ? request.params.arguments
                            : {};
                        if (!toolName) {
                            process.stdout.write(`${JSON.stringify({ id, ok: false, error: { code: "INVALID_REQUEST", message: "call_tool requires params.name" } })}\n`);
                            continue;
                        }
                        try {
                            const result = await this.callTool(toolName, argumentsPayload);
                            process.stdout.write(`${JSON.stringify({ id, ok: true, result })}\n`);
                        }
                        catch (error) {
                            process.stdout.write(`${JSON.stringify({
                                id,
                                ok: false,
                                error: {
                                    code: "CALL_FAILED",
                                    message: error instanceof Error ? error.message : String(error),
                                },
                            })}\n`);
                        }
                        continue;
                    }
                    process.stdout.write(`${JSON.stringify({ id, ok: false, error: { code: "INVALID_REQUEST", message: `Unsupported method '${request.method ?? "unknown"}'.` } })}\n`);
                }
            }
        },
        async close() {
            await sdkServer.close();
            await engineClient.close();
        },
    };
}
