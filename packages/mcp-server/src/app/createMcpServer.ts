import { McpServer as SdkMcpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { readFileSync } from "node:fs";
import { RustEngineClient, type EngineClient } from "../engine/rustEngineClient.js";
import { createFindSymbolService } from "../services/findSymbolService.js";
import { createInspectTreeService } from "../services/inspectTreeService.js";
import { createListEndpointsService } from "../services/listEndpointsService.js";
import { createSearchTextService } from "../services/searchTextService.js";
import { createTraceCallersService } from "../services/traceCallersService.js";
import { createTraceFlowService } from "../services/traceFlowService.js";
import {
  registerCodeTools,
  type RegisteredCodeTool,
} from "../tools/registerCodeTools.js";
import type { ResponseEnvelope } from "../contracts/public/common.js";

export interface CreateMcpServerOptions {
  workspaceRoot: string;
  engineClient?: EngineClient;
}

export const NAVIGATION_MCP_INSTRUCTIONS = [
  "Navigation Agent MCP is a workspace-only structural code navigation server.",
  "Use it before reading files when a task requires code discovery, symbol lookup, route inventory, impact analysis, or flow tracing.",
  "Canonical tools are code.inspect_tree, code.find_symbol, code.list_endpoints, code.search_text, code.trace_flow, and code.trace_callers. Some clients may display them with a server prefix; preserve the code.* semantics.",
  "Supported language filters are typescript, javascript, go, java, php, python, rust, and csharp. Supported framework filters are react-router and spring.",
  "Workflow: inspect_tree for unknown areas; find_symbol when you know a symbol; pass find_symbol items[].path to trace_callers for upstream impact and trace_flow for downstream execution; list_endpoints for route/API surfaces; search_text only for textual patterns or when semantic lookup is not enough.",
  "Fallbacks: if find_symbol misses constants, config, imports, decorators, or textual identifiers, use search_text; if trace output is too broad, narrow path, language, framework, or symbol before reading files.",
  "Do not use this server for web search, external repositories, arbitrary filesystem access, or reading file contents. After it narrows scope, read only the returned files that matter.",
].join(" ");

export const NAVIGATION_MCP_VERSION = readPackageVersion();

export interface McpServerPlan {
  name: "navigation-agent-mcp";
  version: string;
  instructions: string;
  workspaceRoot: string;
  tools: RegisteredCodeTool[];
  listTools(): RegisteredCodeTool[];
  callTool(
    name: string,
    payload: Record<string, unknown>,
  ): Promise<unknown>;
  serveStdio(): Promise<void>;
  serveStdioLegacy(): Promise<void>;
  close(): Promise<void>;
}

function readPackageVersion(): string {
  try {
    const packageJson = JSON.parse(
      readFileSync(new URL("../../package.json", import.meta.url), "utf8"),
    ) as { version?: unknown };

    return typeof packageJson.version === "string" ? packageJson.version : "0.0.0";
  } catch {
    return "0.0.0";
  }
}

function toSdkToolResult(result: ResponseEnvelope<unknown>) {
  return {
    content: [
      {
        type: "text" as const,
        text: JSON.stringify(result, null, 2),
      },
    ],
    structuredContent: result as unknown as Record<string, unknown>,
    isError: result.status === "error",
  };
}

export function createMcpServer(
  options: CreateMcpServerOptions,
): McpServerPlan {
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
  const sdkServer = new SdkMcpServer(
    {
      name: "navigation-agent-mcp",
      version: NAVIGATION_MCP_VERSION,
    },
    {
      instructions: NAVIGATION_MCP_INSTRUCTIONS,
    },
  );

  for (const tool of tools) {
    sdkServer.registerTool(
      tool.name,
      {
        title: tool.title,
        description: tool.description,
        inputSchema: tool.sdkInputSchema,
      },
      async (payload) => toSdkToolResult(await tool.execute(payload as Record<string, unknown>)),
    );
  }

  return {
    name: "navigation-agent-mcp",
    version: NAVIGATION_MCP_VERSION,
    instructions: NAVIGATION_MCP_INSTRUCTIONS,
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
              `${JSON.stringify({ id, ok: true, result: tools.map(({ execute, sdkInputSchema, ...tool }) => tool) })}\n`,
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
      await sdkServer.close();
      await engineClient.close();
    },
  };
}
