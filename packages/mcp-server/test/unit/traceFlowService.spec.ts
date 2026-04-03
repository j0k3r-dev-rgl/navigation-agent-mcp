import assert from "node:assert/strict";
import test from "node:test";

import { createTraceFlowService } from "../../src/services/traceFlowService.js";
import type { EngineClient } from "../../src/engine/rustEngineClient.js";

class MockEngineClient implements EngineClient {
  response: unknown;
  requests: unknown[] = [];

  constructor(response: unknown) {
    this.response = response;
  }

  async request(request: unknown) {
    this.requests.push(request);
    return this.response as never;
  }

  async close() {}
}

test("traceFlowService shapes requests for the engine and preserves the public envelope", async () => {
  const engineClient = new MockEngineClient({
    id: "req-1",
    ok: true,
    result: {
      resolvedPath: "src/routes/dashboard.tsx",
      root: {
        symbol: "loader",
        path: "src/routes/dashboard.tsx",
        kind: "function",
        rangeLine: { init: 5, end: 20 },
        via: null,
        callers: [
          {
            symbol: "fetchData",
            path: "src/shared/api.ts",
            kind: "function",
            rangeLine: { init: 10, end: 15 },
            via: [
              {
                line: 12,
                column: 5,
                snippet: "fetchData()",
                receiverType: null,
              },
            ],
            callers: [],
          },
        ],
      },
      truncated: false,
    },
  });
  const service = createTraceFlowService({
    workspaceRoot: "/workspace",
    engineClient,
  });

  const result = await service.validateAndExecute({
    path: "src/routes/dashboard.tsx",
    symbol: "loader",
    framework: "react-router",
  });

  assert.deepEqual(engineClient.requests, [
    {
      id: "req-1",
      capability: "workspace.trace_flow",
      workspaceRoot: "/workspace",
      payload: {
        path: "src/routes/dashboard.tsx",
        symbol: "loader",
        analyzerLanguage: "typescript",
        publicLanguageFilter: "typescript",
      },
    },
  ]);
  assert.equal(result.status, "ok");
  assert.equal(result.summary, "Traced 1 callee for 'loader' from 'src/routes/dashboard.tsx'.");
  assert.deepEqual(result.data.entrypoint, {
    path: "src/routes/dashboard.tsx",
    symbol: "loader",
    language: "typescript",
  });
  assert.equal(result.data.root?.callers.length, 1);
  assert.equal(result.data.root?.callers[0]?.symbol, "fetchData");
  assert.deepEqual(result.meta.counts, { returnedCount: 1, totalMatched: 1 });
});

test("traceFlowService maps path and unsupported-capability failures to stable responses", async () => {
  const missingPathService = createTraceFlowService({
    workspaceRoot: "/workspace",
    engineClient: new MockEngineClient({
      id: "req-1",
      ok: false,
      error: {
        code: "FILE_NOT_FOUND",
        message: "Path 'missing.ts' was not found inside the configured workspace root.",
        retryable: false,
        details: { path: "missing.ts" },
      },
    }),
  });

  const missingPathResult = await missingPathService.validateAndExecute({
    path: "missing.ts",
    symbol: "loader",
  });
  assert.equal(missingPathResult.status, "error");
  assert.equal(missingPathResult.summary, "Path not found.");
  assert.equal(missingPathResult.errors[0]?.code, "FILE_NOT_FOUND");

  const unsupportedService = createTraceFlowService({
    workspaceRoot: "/workspace",
    engineClient: new MockEngineClient({
      id: "req-2",
      ok: false,
      error: {
        code: "UNSUPPORTED_CAPABILITY",
        message: "Capability 'workspace.trace_flow' is not implemented yet.",
        retryable: false,
        details: { capability: "workspace.trace_flow" },
      },
    }),
  });

  const unsupportedResult = await unsupportedService.validateAndExecute({
    path: "src/routes/dashboard.tsx",
    symbol: "loader",
  });
  assert.equal(unsupportedResult.status, "error");
  assert.equal(unsupportedResult.summary, "Flow trace failed.");
  assert.equal(unsupportedResult.errors[0]?.code, "BACKEND_EXECUTION_FAILED");
});
