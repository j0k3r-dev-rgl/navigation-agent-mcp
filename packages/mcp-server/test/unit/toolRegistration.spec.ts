import test from "node:test";
import assert from "node:assert/strict";

import { createMcpServer } from "../../src/app/createMcpServer.ts";
import type { EngineClient } from "../../src/engine/rustEngineClient.ts";
import type { PythonFallbackBridge } from "../../src/runtime/pythonFallback.ts";

class MockEngineClient implements EngineClient {
  requests: Array<{ capability: string; payload: unknown }> = [];

  async request(request: { capability: string; payload: unknown }) {
    this.requests.push(request);

    if (request.capability === "workspace.find_symbol") {
      return {
        id: "req-1",
        ok: true as const,
        result: {
          resolvedPath: null,
          items: [],
          totalMatched: 0,
          truncated: false,
        },
      };
    }

    if (request.capability === "workspace.search_text") {
      return {
        id: "req-1",
        ok: true as const,
        result: {
          resolvedPath: null,
          items: [],
          totalFileCount: 0,
          totalMatchCount: 0,
          truncated: false,
        },
      };
    }

    if (request.capability === "workspace.trace_symbol") {
      return {
        id: "req-1",
        ok: true as const,
        result: {
          resolvedPath: "src/index.ts",
          items: [
            {
              path: "src/index.ts",
              language: "typescript",
            },
          ],
          totalMatched: 1,
          truncated: false,
        },
      };
    }

    if (request.capability === "workspace.trace_callers") {
      return {
        id: "req-1",
        ok: true as const,
        result: {
          resolvedPath: "src/index.ts",
          items: [
            {
              path: "src/routes/layout.tsx",
              line: 12,
              column: 3,
              caller: "Layout",
              callerSymbol: "Layout",
              relation: "calls",
              language: "typescript",
              snippet: "loader()",
              receiverType: null,
            },
          ],
          totalMatched: 1,
          truncated: false,
          recursive: null,
        },
      };
    }

    return {
      id: "req-1",
      ok: true as const,
      result: {
        root: ".",
        items: [],
        truncated: false,
        maxItems: 2000,
        ignoredDirectories: [],
      },
    };
  }

  async close() {}
}

class MockFallbackBridge implements PythonFallbackBridge {
  readonly workspaceRoot = "/workspace";
  calls: string[] = [];

  async execute(toolName: string) {
    this.calls.push(toolName);
    return {
      tool: toolName,
      status: "ok" as const,
      summary: `Executed ${toolName}`,
      data: { ok: true },
      errors: [],
      meta: {
        query: {},
        resolvedPath: null,
        truncated: false,
        counts: {},
        detection: {},
      },
    };
  }
}

test("registers the stable six code tools with expected schema defaults", () => {
  const server = createMcpServer({
    workspaceRoot: "/workspace",
    engineClient: new MockEngineClient(),
    fallbackBridge: new MockFallbackBridge(),
  });

  const tools = server.listTools();
  const toolsByName = Object.fromEntries(tools.map((tool) => [tool.name, tool]));

  assert.deepEqual(new Set(Object.keys(toolsByName)), new Set([
    "code.find_symbol",
    "code.search_text",
    "code.trace_symbol",
    "code.trace_callers",
    "code.list_endpoints",
    "code.inspect_tree",
  ]));

  assert.equal(
    (toolsByName["code.inspect_tree"].inputSchema.properties as Record<string, { default?: unknown }>).max_depth.default,
    3,
  );
  assert.equal(
    (toolsByName["code.inspect_tree"].inputSchema.properties as Record<string, { maximum?: unknown }>).max_depth.maximum,
    20,
  );
  assert.deepEqual(toolsByName["code.find_symbol"].inputSchema.required, ["symbol"]);
  assert.ok("MatchMode" in (toolsByName["code.find_symbol"].inputSchema.$defs as Record<string, unknown>));
  assert.deepEqual(new Set(toolsByName["code.trace_symbol"].inputSchema.required), new Set(["path", "symbol"]));

  const maxDepthSchema = ((toolsByName["code.trace_callers"].inputSchema.properties as Record<string, { anyOf: Array<Record<string, unknown>> }>).max_depth.anyOf[0]);
  assert.equal(maxDepthSchema.minimum, 1);
  assert.equal(maxDepthSchema.maximum, 8);
  assert.equal(
    (toolsByName["code.list_endpoints"].inputSchema.properties as Record<string, { default?: unknown }>).limit.default,
    50,
  );
  assert.ok("PublicEndpointKind" in (toolsByName["code.list_endpoints"].inputSchema.$defs as Record<string, unknown>));
});

test("migrated and compatibility tools route through the expected backend", async () => {
  const engineClient = new MockEngineClient();
  const fallbackBridge = new MockFallbackBridge();
  const server = createMcpServer({
    workspaceRoot: "/workspace",
    engineClient,
    fallbackBridge,
  });

  const result = await server.callTool("code.search_text", { query: "inspect_tree" });
  assert.equal((result as { tool: string }).tool, "code.search_text");
  assert.equal(engineClient.requests[0]?.capability, "workspace.search_text");
  assert.deepEqual(fallbackBridge.calls, []);

  const findSymbolResult = await server.callTool("code.find_symbol", {
    symbol: "loader",
  });
  assert.equal((findSymbolResult as { tool: string }).tool, "code.find_symbol");
  assert.equal(engineClient.requests[1]?.capability, "workspace.find_symbol");

  const traceResult = await server.callTool("code.trace_symbol", {
    path: "src/index.ts",
    symbol: "loader",
  });
  assert.equal((traceResult as { tool: string }).tool, "code.trace_symbol");
  assert.equal(engineClient.requests[2]?.capability, "workspace.trace_symbol");

  const callerTraceResult = await server.callTool("code.trace_callers", {
    path: "src/index.ts",
    symbol: "loader",
  });
  assert.equal((callerTraceResult as { tool: string }).tool, "code.trace_callers");
  assert.equal(engineClient.requests[3]?.capability, "workspace.trace_callers");
  assert.deepEqual(fallbackBridge.calls, []);
});
