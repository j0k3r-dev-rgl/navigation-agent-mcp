import assert from "node:assert/strict";
import test from "node:test";

import { createFindSymbolService } from "../../src/services/findSymbolService.js";
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

test("findSymbolService shapes framework, javascript, python, and rust requests for the engine", async () => {
  const engineClient = new MockEngineClient({
    id: "req-1",
    ok: true,
    result: {
      resolvedPath: null,
      items: [],
      totalMatched: 0,
      truncated: false,
    },
  });
  const service = createFindSymbolService({
    workspaceRoot: "/workspace",
    engineClient,
  });

  await service.validateAndExecute({ symbol: "loader", framework: "react-router" });
  await service.validateAndExecute({ symbol: "loader", language: "javascript" });
  await service.validateAndExecute({ symbol: "ExampleService", framework: "spring" });
  await service.validateAndExecute({ symbol: "fetch_users", language: "python" });
  await service.validateAndExecute({ symbol: "AnalyzerRegistry", language: "rust" });

  assert.deepEqual(
    engineClient.requests.map((request) => {
      const value = request as {
        capability: string;
        workspaceRoot: string;
        payload: unknown;
      };

      return {
        capability: value.capability,
        workspaceRoot: value.workspaceRoot,
        payload: value.payload,
      };
    }),
    [
    {
      capability: "workspace.find_symbol",
      workspaceRoot: "/workspace",
      payload: {
        symbol: "loader",
        path: null,
        analyzerLanguage: "typescript",
        publicLanguageFilter: "typescript",
        kind: "any",
        matchMode: "exact",
        limit: 50,
      },
    },
    {
      capability: "workspace.find_symbol",
      workspaceRoot: "/workspace",
      payload: {
        symbol: "loader",
        path: null,
        analyzerLanguage: "typescript",
        publicLanguageFilter: "javascript",
        kind: "any",
        matchMode: "exact",
        limit: 50,
      },
    },
    {
      capability: "workspace.find_symbol",
      workspaceRoot: "/workspace",
      payload: {
        symbol: "ExampleService",
        path: null,
        analyzerLanguage: "java",
        publicLanguageFilter: "java",
        kind: "any",
        matchMode: "exact",
        limit: 50,
      },
    },
    {
      capability: "workspace.find_symbol",
      workspaceRoot: "/workspace",
      payload: {
        symbol: "fetch_users",
        path: null,
        analyzerLanguage: "python",
        publicLanguageFilter: "python",
        kind: "any",
        matchMode: "exact",
        limit: 50,
      },
    },
    {
      capability: "workspace.find_symbol",
      workspaceRoot: "/workspace",
      payload: {
        symbol: "AnalyzerRegistry",
        path: null,
        analyzerLanguage: "rust",
        publicLanguageFilter: "rust",
        kind: "any",
        matchMode: "exact",
        limit: 50,
      },
    },
  ],
  );
});

test("findSymbolService maps missing-path errors to the stable public response", async () => {
  const service = createFindSymbolService({
    workspaceRoot: "/workspace",
    engineClient: new MockEngineClient({
      id: "req-1",
      ok: false,
      error: {
        code: "FILE_NOT_FOUND",
        message: "Path 'missing' was not found inside the configured workspace root.",
        retryable: false,
        details: { path: "missing" },
      },
    }),
  });

  const result = await service.validateAndExecute({ symbol: "loader", path: "missing" });
  assert.equal(result.status, "error");
  assert.equal(result.summary, "Path not found.");
  assert.equal(result.errors[0]?.code, "FILE_NOT_FOUND");
  assert.deepEqual(result.errors[0]?.details, { path: "missing" });
});

test("findSymbolService preserves partial summaries and truncation metadata", async () => {
  const service = createFindSymbolService({
    workspaceRoot: "/workspace",
    engineClient: new MockEngineClient({
      id: "req-1",
      ok: true,
      result: {
        resolvedPath: "src",
        items: [
          {
            symbol: "loader",
            kind: "function",
            path: "src/routes/a.ts",
            line: 10,
            lineEnd: 12,
            language: "typescript",
          },
        ],
        totalMatched: 3,
        truncated: true,
      },
    }),
  });

  const result = await service.validateAndExecute({
    symbol: "loader",
    path: "src",
    limit: 1,
  });

  assert.equal(result.status, "partial");
  assert.equal(
    result.summary,
    "Found 3 symbol definitions for 'loader' and returned a truncated subset.",
  );
  assert.equal(result.data.count, 3);
  assert.equal(result.data.returnedCount, 1);
  assert.equal(result.meta.truncated, true);
  assert.deepEqual(result.meta.counts, { returnedCount: 1, totalMatched: 3 });
  assert.equal(result.errors[0]?.code, "RESULT_TRUNCATED");
});

test("findSymbolService maps unsupported capability to backend execution failure", async () => {
  const service = createFindSymbolService({
    workspaceRoot: "/workspace",
    engineClient: new MockEngineClient({
      id: "req-1",
      ok: false,
      error: {
        code: "UNSUPPORTED_CAPABILITY",
        message: "Capability 'workspace.find_symbol' is not implemented yet.",
        retryable: false,
        details: { capability: "workspace.find_symbol" },
      },
    }),
  });

  const result = await service.validateAndExecute({ symbol: "loader" });
  assert.equal(result.status, "error");
  assert.equal(result.summary, "Symbol analysis failed.");
  assert.equal(result.errors[0]?.code, "BACKEND_EXECUTION_FAILED");
});