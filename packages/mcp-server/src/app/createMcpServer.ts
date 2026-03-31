import {
  createPythonFallbackBridge,
  type PythonFallbackBridge,
} from "../runtime/pythonFallback.ts";
import { RustEngineClient, type EngineClient } from "../engine/rustEngineClient.ts";
import { createFindSymbolService } from "../services/findSymbolService.ts";
import { createInspectTreeService } from "../services/inspectTreeService.ts";
import { createListEndpointsService } from "../services/listEndpointsService.ts";
import { createSearchTextService } from "../services/searchTextService.ts";
import { createTraceCallersService } from "../services/traceCallersService.ts";
import { createTraceSymbolService } from "../services/traceSymbolService.ts";
import {
  registerCodeTools,
  type RegisteredCodeTool,
} from "../tools/registerCodeTools.ts";

export interface CreateMcpServerOptions {
  workspaceRoot: string;
  fallbackBridge?: PythonFallbackBridge;
  engineClient?: EngineClient;
}

export interface McpServerPlan {
  name: "navigation-agent-mcp";
  workspaceRoot: string;
  tools: RegisteredCodeTool[];
  listTools(): RegisteredCodeTool[];
  callTool(
    name: string,
    payload: Record<string, unknown>,
  ): Promise<unknown>;
  serveStdio(): Promise<void>;
  close(): Promise<void>;
}

export function createMcpServer(
  options: CreateMcpServerOptions,
): McpServerPlan {
  const fallbackBridge =
    options.fallbackBridge ?? createPythonFallbackBridge({ workspaceRoot: options.workspaceRoot });
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
  const traceSymbolService = createTraceSymbolService({
    workspaceRoot: options.workspaceRoot,
    engineClient,
  });
  const traceCallersService = createTraceCallersService({
    workspaceRoot: options.workspaceRoot,
    engineClient,
  });

  const tools = registerCodeTools({
    fallbackBridge,
    inspectTreeHandler: (payload) => inspectTreeService.validateAndExecute(payload),
    findSymbolHandler: (payload) => findSymbolService.validateAndExecute(payload),
    listEndpointsHandler: (payload) => listEndpointsService.validateAndExecute(payload),
    searchTextHandler: (payload) => searchTextService.validateAndExecute(payload),
    traceCallersHandler: (payload) => traceCallersService.validateAndExecute(payload),
    traceSymbolHandler: (payload) => traceSymbolService.validateAndExecute(payload),
  });

  return {
    name: "navigation-agent-mcp",
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

          let request: { id?: string | number; method?: string; params?: Record<string, unknown> };
          try {
            request = JSON.parse(trimmed) as {
              id?: string | number;
              method?: string;
              params?: Record<string, unknown>;
            };
          } catch (error) {
            process.stdout.write(
              `${JSON.stringify({
                id: null,
                ok: false,
                error: {
                  code: "INVALID_REQUEST",
                  message: error instanceof Error ? error.message : String(error),
                },
              })}\n`,
            );
            continue;
          }

          const id = request.id ?? null;
          if (request.method === "list_tools") {
            process.stdout.write(
              `${JSON.stringify({ id, ok: true, result: tools.map(({ execute, ...tool }) => tool) })}\n`,
            );
            continue;
          }

          if (request.method === "call_tool") {
            const toolName = typeof request.params?.name === "string" ? request.params.name : null;
            const argumentsPayload =
              request.params && typeof request.params.arguments === "object" && request.params.arguments
                ? (request.params.arguments as Record<string, unknown>)
                : {};

            if (!toolName) {
              process.stdout.write(
                `${JSON.stringify({ id, ok: false, error: { code: "INVALID_REQUEST", message: "call_tool requires params.name" } })}\n`,
              );
              continue;
            }

            try {
              const result = await this.callTool(toolName, argumentsPayload);
              process.stdout.write(`${JSON.stringify({ id, ok: true, result })}\n`);
            } catch (error) {
              process.stdout.write(
                `${JSON.stringify({
                  id,
                  ok: false,
                  error: {
                    code: "CALL_FAILED",
                    message: error instanceof Error ? error.message : String(error),
                  },
                })}\n`,
              );
            }
            continue;
          }

          process.stdout.write(
            `${JSON.stringify({ id, ok: false, error: { code: "INVALID_REQUEST", message: `Unsupported method '${request.method ?? "unknown"}'.` } })}\n`,
          );
        }
      }
    },
    async close() {
      await engineClient.close();
    },
  };
}
