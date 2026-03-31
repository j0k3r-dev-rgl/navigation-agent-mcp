import assert from "node:assert/strict";
import test from "node:test";

import { createTraceSymbolService } from "../../src/services/traceSymbolService.ts";
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

test("traceSymbolService shapes requests for the engine and preserves the public envelope", async () => {
  const engineClient = new MockEngineClient({
    id: "req-1",
    ok: true,
    result: {
      resolvedPath: "src/routes/dashboard.tsx",
      items: [
        { path: "back/src/main/java/com/acme/HomeController.java", language: "java" },
        { path: "src/routes/dashboard.tsx", language: "typescript" },
      ],
      totalMatched: 2,
      truncated: false,
    },
  });
  const service = createTraceSymbolService({
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
      capability: "workspace.trace_symbol",
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
  assert.equal(result.summary, "Traced 2 related files for 'loader' from 'src/routes/dashboard.tsx'.");
  assert.deepEqual(result.data.entrypoint, {
    path: "src/routes/dashboard.tsx",
    symbol: "loader",
    language: "typescript",
  });
  assert.equal(result.data.fileCount, 2);
  assert.deepEqual(result.meta.counts, { returnedCount: 2, totalMatched: 2 });
});

test("traceSymbolService maps path and unsupported-capability failures to stable responses", async () => {
  const missingPathService = createTraceSymbolService({
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

  const unsupportedService = createTraceSymbolService({
    workspaceRoot: "/workspace",
    engineClient: new MockEngineClient({
      id: "req-2",
      ok: false,
      error: {
        code: "UNSUPPORTED_CAPABILITY",
        message: "Capability 'workspace.trace_symbol' is not implemented yet.",
        retryable: false,
        details: { capability: "workspace.trace_symbol" },
      },
    }),
  });

  const unsupportedResult = await unsupportedService.validateAndExecute({
    path: "src/routes/dashboard.tsx",
    symbol: "loader",
  });
  assert.equal(unsupportedResult.status, "error");
  assert.equal(unsupportedResult.summary, "Symbol trace failed.");
  assert.equal(unsupportedResult.errors[0]?.code, "BACKEND_EXECUTION_FAILED");
});
