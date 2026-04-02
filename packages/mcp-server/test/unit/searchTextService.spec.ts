import assert from "node:assert/strict";
import test from "node:test";

import { createSearchTextService } from "../../src/services/searchTextService.js";
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

test("searchTextService validates input and shapes engine requests", async () => {
  const engineClient = new MockEngineClient({
    id: "req-1",
    ok: true,
    result: {
      resolvedPath: null,
      items: [],
      totalFileCount: 0,
      totalMatchCount: 0,
      truncated: false,
    },
  });
  const service = createSearchTextService({
    workspaceRoot: "/workspace",
    engineClient,
  });

  await service.validateAndExecute({ query: "loader", framework: "react-router" });
  await service.validateAndExecute({ query: "ExampleService", framework: "spring" });
  await service.validateAndExecute({ query: "fetch_users", language: "python", regex: true, context: 2, limit: 10 });

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
        capability: "workspace.search_text",
        workspaceRoot: "/workspace",
        payload: {
          query: "loader",
          path: null,
          publicLanguageFilter: "typescript",
          include: null,
          regex: false,
          context: 1,
          limit: 50,
        },
      },
      {
        capability: "workspace.search_text",
        workspaceRoot: "/workspace",
        payload: {
          query: "ExampleService",
          path: null,
          publicLanguageFilter: "java",
          include: null,
          regex: false,
          context: 1,
          limit: 50,
        },
      },
      {
        capability: "workspace.search_text",
        workspaceRoot: "/workspace",
        payload: {
          query: "fetch_users",
          path: null,
          publicLanguageFilter: "python",
          include: null,
          regex: true,
          context: 2,
          limit: 10,
        },
      },
    ],
  );
});

test("searchTextService preserves the stable public envelope for partial results", async () => {
  const service = createSearchTextService({
    workspaceRoot: "/workspace",
    engineClient: new MockEngineClient({
      id: "req-1",
      ok: true,
      result: {
        resolvedPath: "src",
        items: [
          {
            path: "src/routes/a.ts",
            language: "typescript",
            matchCount: 2,
            matches: [
              {
                line: 10,
                text: "export async function loader() {}",
                submatches: [{ start: 22, end: 28, text: "loader" }],
                before: [{ line: 9, text: "" }],
                after: [{ line: 11, text: "return null;" }],
              },
              {
                line: 20,
                text: "const loaderState = true;",
                submatches: [{ start: 6, end: 12, text: "loader" }],
                before: [],
                after: [],
              },
            ],
          },
        ],
        totalFileCount: 3,
        totalMatchCount: 5,
        truncated: true,
      },
    }),
  });

  const result = await service.validateAndExecute({
    query: "loader",
    path: "src",
    limit: 1,
  });

  assert.equal(result.status, "partial");
  assert.equal(
    result.summary,
    "Found 5 text matches across 3 files for 'loader' and returned a truncated subset.",
  );
  assert.deepEqual(result.data, {
    fileCount: 1,
    matchCount: 2,
    totalFileCount: 3,
    totalMatchCount: 5,
    items: [
      {
        path: "src/routes/a.ts",
        language: "typescript",
        matchCount: 2,
        matches: [
          {
            line: 10,
            text: "export async function loader() {}",
            submatches: [{ start: 22, end: 28, text: "loader" }],
            before: [{ line: 9, text: "" }],
            after: [{ line: 11, text: "return null;" }],
          },
          {
            line: 20,
            text: "const loaderState = true;",
            submatches: [{ start: 6, end: 12, text: "loader" }],
            before: [],
            after: [],
          },
        ],
      },
    ],
  });
  assert.deepEqual(result.meta, {
    query: {
      query: "loader",
      path: "src",
      language: null,
      framework: null,
      include: null,
      regex: false,
      context: 1,
      limit: 1,
    },
    resolvedPath: "src",
    truncated: true,
    counts: {
      returnedFileCount: 1,
      totalFileCount: 3,
      returnedMatchCount: 2,
      totalMatchCount: 5,
    },
    detection: {
      effectiveLanguage: null,
      framework: null,
    },
  });
  assert.equal(result.errors[0]?.code, "RESULT_TRUNCATED");
});

test("searchTextService preserves backend dependency errors", async () => {
  const service = createSearchTextService({
    workspaceRoot: "/workspace",
    engineClient: new MockEngineClient({
      id: "req-1",
      ok: false,
      error: {
        code: "BACKEND_DEPENDENCY_NOT_FOUND",
        message: "ripgrep (rg) is required for text search but is not installed.",
        retryable: false,
        details: { dependency: "rg" },
      },
    }),
  });

  const result = await service.validateAndExecute({ query: "loader" });
  assert.equal(result.status, "error");
  assert.equal(result.summary, "Text search failed.");
  assert.equal(result.errors[0]?.code, "BACKEND_DEPENDENCY_NOT_FOUND");
  assert.deepEqual(result.errors[0]?.details, { dependency: "rg" });
});
