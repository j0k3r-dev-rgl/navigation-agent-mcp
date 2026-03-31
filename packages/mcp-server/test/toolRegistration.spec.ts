import test from "node:test";
import assert from "node:assert/strict";

import { createMcpServer } from "../src/app/createMcpServer.ts";
import type { EngineClient } from "../src/engine/rustEngineClient.ts";
import type { PythonFallbackBridge } from "../src/runtime/pythonFallback.ts";

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

test("non-migrated tools still route through the compatibility bridge", async () => {
  const engineClient = new MockEngineClient();
  const fallbackBridge = new MockFallbackBridge();
  const server = createMcpServer({
    workspaceRoot: "/workspace",
    engineClient,
    fallbackBridge,
  });

  const result = await server.callTool("code.search_text", { query: "inspect_tree" });
  assert.equal((result as { tool: string }).tool, "code.search_text");
  assert.deepEqual(fallbackBridge.calls, ["code.search_text"]);

  const findSymbolResult = await server.callTool("code.find_symbol", {
    symbol: "loader",
  });
  assert.equal((findSymbolResult as { tool: string }).tool, "code.find_symbol");
  assert.equal(engineClient.requests[0]?.capability, "workspace.find_symbol");
  assert.deepEqual(fallbackBridge.calls, ["code.search_text"]);
});
