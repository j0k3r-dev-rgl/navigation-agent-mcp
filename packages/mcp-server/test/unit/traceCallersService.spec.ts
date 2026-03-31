import assert from "node:assert/strict";
import test from "node:test";

import { createTraceCallersService } from "../../src/services/traceCallersService.ts";
import type { EngineClient } from "../../src/engine/rustEngineClient.ts";

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

test("traceCallersService shapes requests for the engine and preserves the public envelope", async () => {
  const engineClient = new MockEngineClient({
    id: "req-1",
    ok: true,
    result: {
      resolvedPath: "src/routes/dashboard.tsx",
      items: [
        {
          path: "src/routes/layout.tsx",
          line: 9,
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
  });
  const service = createTraceCallersService({
    workspaceRoot: "/workspace",
    engineClient,
  });

  const result = await service.validateAndExecute({
    path: "src/routes/dashboard.tsx",
    symbol: "loader",
    framework: "react-router",
    recursive: true,
    max_depth: 4,
  });

  assert.deepEqual(engineClient.requests, [
    {
      id: "req-1",
      capability: "workspace.trace_callers",
      workspaceRoot: "/workspace",
      payload: {
        path: "src/routes/dashboard.tsx",
        symbol: "loader",
        analyzerLanguage: "typescript",
        publicLanguageFilter: "typescript",
        recursive: true,
        maxDepth: 4,
      },
    },
  ]);
  assert.equal(result.status, "ok");
  assert.equal(
    result.summary,
    "Found 1 incoming caller for 'loader' from 'src/routes/dashboard.tsx' with recursive reverse trace.",
  );
  assert.equal(result.data.count, 1);
  assert.equal(result.data.items[0]?.callerSymbol, "Layout");
  assert.equal(result.meta.counts.totalMatched, 1);
});

test("traceCallersService maps stable error responses", async () => {
  const missingPathService = createTraceCallersService({
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

  const missingResult = await missingPathService.validateAndExecute({
    path: "missing.ts",
    symbol: "loader",
  });
  assert.equal(missingResult.status, "error");
  assert.equal(missingResult.summary, "Path not found.");
  assert.equal(missingResult.errors[0]?.code, "FILE_NOT_FOUND");

  const unsupportedService = createTraceCallersService({
    workspaceRoot: "/workspace",
    engineClient: new MockEngineClient({
      id: "req-2",
      ok: false,
      error: {
        code: "UNSUPPORTED_CAPABILITY",
        message: "Capability 'workspace.trace_callers' is not implemented yet.",
        retryable: false,
        details: { capability: "workspace.trace_callers" },
      },
    }),
  });

  const unsupportedResult = await unsupportedService.validateAndExecute({
    path: "src/routes/dashboard.tsx",
    symbol: "loader",
  });
  assert.equal(unsupportedResult.status, "error");
  assert.equal(unsupportedResult.summary, "Caller trace failed.");
  assert.equal(unsupportedResult.errors[0]?.code, "BACKEND_EXECUTION_FAILED");
});
