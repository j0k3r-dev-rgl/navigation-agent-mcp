import test from "node:test";
import assert from "node:assert/strict";
import os from "node:os";
import path from "node:path";
import { promises as fs } from "node:fs";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";

const repoRoot = "/home/j0k3r/navigation-agent-mcp";

test("TS find_symbol preserves the public contract while adding Python and Rust", async () => {
  const tempDir = await fs.mkdtemp(path.join(os.tmpdir(), "nav-mcp-find-symbol-contract-"));
  const engineScriptPath = path.join(tempDir, "engine_find_symbol_stub.py");
  await fs.writeFile(engineScriptPath, buildFindSymbolEngineStubScript(), "utf8");

  const { client, transport } = await createSdkClient(tempDir, {
    NAVIGATION_MCP_RUST_ENGINE_CMD: JSON.stringify(["python", engineScriptPath]),
  });

  try {
    const exactResponse = extractEnvelope(
      await client.callTool({
        name: "code.find_symbol",
        arguments: {
          symbol: "UserRestController",
          language: "java",
          match: "exact",
          limit: 5,
        },
      }),
    );

    assert.equal(exactResponse.tool, "code.find_symbol");
    assert.equal(exactResponse.status, "ok");
    assert.equal(
      exactResponse.summary,
      "Found 1 symbol definition for 'UserRestController'.",
    );
    assert.deepEqual(exactResponse.data, {
      count: 1,
      returnedCount: 1,
      totalMatched: 1,
      items: [
        {
          symbol: "UserRestController",
          kind: "class",
          path: "src/main/java/com/example/UserRestController.java",
          line: 12,
          language: "java",
        },
      ],
    });
    assert.deepEqual(exactResponse.meta, {
      query: {
        symbol: "UserRestController",
        language: "java",
        framework: null,
        kind: "any",
        match: "exact",
        path: null,
        limit: 5,
      },
      resolvedPath: null,
      truncated: false,
      counts: {
        returnedCount: 1,
        totalMatched: 1,
      },
      detection: {
        effectiveLanguage: "java",
        framework: null,
      },
    });

    const partialResponse = extractEnvelope(
      await client.callTool({
        name: "code.find_symbol",
        arguments: {
          symbol: "Titular",
          language: "java",
          kind: "class",
          match: "fuzzy",
          limit: 2,
        },
      }),
    );

    assert.equal(partialResponse.tool, "code.find_symbol");
    assert.equal(partialResponse.status, "partial");
    assert.equal(
      partialResponse.summary,
      "Found 3 symbol definitions for 'Titular' and returned a truncated subset.",
    );
    assert.equal(partialResponse.meta.truncated, true);
    assert.deepEqual(partialResponse.meta.counts, {
      returnedCount: 2,
      totalMatched: 3,
    });
    assert.equal(partialResponse.errors[0].code, "RESULT_TRUNCATED");
    assert.deepEqual(
      partialResponse.data.items.map(
      (item: { path: string; line: number; symbol: string; kind: string; language: string }) => ({
        path: item.path,
        line: item.line,
        symbol: item.symbol,
        kind: item.kind,
        language: item.language,
      }),
    ),
    [
      {
        path: "src/main/java/com/example/GetTitularByIdUseCase.java",
        line: 5,
        symbol: "GetTitularByIdUseCase",
        kind: "class",
        language: "java",
      },
      {
        path: "src/main/java/com/example/TitularResponse.java",
        line: 9,
        symbol: "TitularResponse",
        kind: "class",
        language: "java",
      },
    ],
  );

    const pythonResponse = extractEnvelope(
      await client.callTool({
        name: "code.find_symbol",
        arguments: {
          symbol: "fetch_users",
          language: "python",
          match: "exact",
          limit: 5,
        },
      }),
    );

    const rustResponse = extractEnvelope(
      await client.callTool({
        name: "code.find_symbol",
        arguments: {
          symbol: "AnalyzerRegistry",
          language: "rust",
          match: "exact",
          limit: 5,
        },
      }),
    );

    assert.equal(pythonResponse.tool, "code.find_symbol");
    assert.equal(pythonResponse.status, "ok");
    assert.equal(
      pythonResponse.summary,
      "Found 1 symbol definition for 'fetch_users'.",
    );
    assert.deepEqual(pythonResponse.data, {
      count: 1,
      returnedCount: 1,
      totalMatched: 1,
      items: [
        {
          symbol: "fetch_users",
          kind: "function",
          path: "profiles/service.py",
          line: 7,
          language: "python",
        },
      ],
    });
    assert.deepEqual(pythonResponse.meta, {
      query: {
        symbol: "fetch_users",
        language: "python",
        framework: null,
        kind: "any",
        match: "exact",
        path: null,
        limit: 5,
      },
      resolvedPath: null,
      truncated: false,
      counts: {
        returnedCount: 1,
        totalMatched: 1,
      },
      detection: {
        effectiveLanguage: "python",
        framework: null,
      },
    });
    assert.deepEqual(pythonResponse.errors, []);

    assert.equal(rustResponse.tool, "code.find_symbol");
    assert.equal(rustResponse.status, "ok");
    assert.equal(
      rustResponse.summary,
      "Found 1 symbol definition for 'AnalyzerRegistry'.",
    );
    assert.deepEqual(rustResponse.data, {
      count: 1,
      returnedCount: 1,
      totalMatched: 1,
      items: [
        {
          symbol: "AnalyzerRegistry",
          kind: "type",
          path: "crates/navigation-engine/src/analyzers/registry.rs",
          line: 11,
          language: "rust",
        },
      ],
    });
    assert.deepEqual(rustResponse.meta, {
      query: {
        symbol: "AnalyzerRegistry",
        language: "rust",
        framework: null,
        kind: "any",
        match: "exact",
        path: null,
        limit: 5,
      },
      resolvedPath: null,
      truncated: false,
      counts: {
        returnedCount: 1,
        totalMatched: 1,
      },
      detection: {
        effectiveLanguage: "rust",
        framework: null,
      },
    });
    assert.deepEqual(rustResponse.errors, []);
  } finally {
    await transport.close();
  }
});

function buildFindSymbolEngineStubScript(): string {
  return `
import json
import sys

for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    request = json.loads(line)
    payload = request["payload"]

    if request["capability"] != "workspace.find_symbol":
        response = {"id": request["id"], "ok": False, "error": {"code": "UNSUPPORTED_CAPABILITY", "message": "Unsupported capability", "retryable": False, "details": {}}}
        print(json.dumps(response), flush=True)
        continue

    if payload.get("symbol") == "UserRestController":
        response = {
            "id": request["id"],
            "ok": True,
            "result": {
                "resolvedPath": None,
                "items": [{
                    "symbol": "UserRestController",
                    "kind": "class",
                    "path": "src/main/java/com/example/UserRestController.java",
                    "line": 12,
                    "language": "java"
                }],
                "totalMatched": 1,
                "truncated": False
            }
        }
        print(json.dumps(response), flush=True)
        continue

    if payload.get("symbol") == "fetch_users":
        response = {
            "id": request["id"],
            "ok": True,
            "result": {
                "resolvedPath": None,
                "items": [{
                    "symbol": "fetch_users",
                    "kind": "function",
                    "path": "profiles/service.py",
                    "line": 7,
                    "language": "python"
                }],
                "totalMatched": 1,
                "truncated": False
            }
        }
        print(json.dumps(response), flush=True)
        continue

    if payload.get("symbol") == "AnalyzerRegistry":
        response = {
            "id": request["id"],
            "ok": True,
            "result": {
                "resolvedPath": None,
                "items": [{
                    "symbol": "AnalyzerRegistry",
                    "kind": "type",
                    "path": "crates/navigation-engine/src/analyzers/registry.rs",
                    "line": 11,
                    "language": "rust"
                }],
                "totalMatched": 1,
                "truncated": False
            }
        }
        print(json.dumps(response), flush=True)
        continue

    response = {
        "id": request["id"],
        "ok": True,
        "result": {
            "resolvedPath": None,
            "items": [
                {"symbol": "GetTitularByIdUseCase", "kind": "class", "path": "src/main/java/com/example/GetTitularByIdUseCase.java", "line": 5, "language": "java"},
                {"symbol": "TitularResponse", "kind": "class", "path": "src/main/java/com/example/TitularResponse.java", "line": 9, "language": "java"}
            ],
            "totalMatched": 3,
            "truncated": True
        }
    }
    print(json.dumps(response), flush=True)
`;
}

async function createSdkClient(
  workspaceRoot: string,
  extraEnv: Record<string, string>,
) {
  const client = new Client(
    { name: "navigation-agent-contract-tests", version: "1.0.0" },
    { capabilities: {} },
  );
  const transport = new StdioClientTransport({
    command: "node",
    args: [
      "--experimental-strip-types",
      "packages/mcp-server/src/bin/navigation-mcp.ts",
      "--transport",
      "stdio",
      "--workspace-root",
      workspaceRoot,
    ],
    cwd: repoRoot,
    env: {
      ...Object.fromEntries(
        Object.entries(process.env).filter(
          (entry): entry is [string, string] => typeof entry[1] === "string",
        ),
      ),
      ...extraEnv,
    },
    stderr: "pipe",
  });

  await client.connect(transport);

  return { client, transport };
}

function extractEnvelope(result: { structuredContent?: unknown; content?: Array<{ type: string; text?: string }> }) {
  if (result.structuredContent && typeof result.structuredContent === "object") {
    return result.structuredContent as Record<string, unknown>;
  }

  const textBlock = result.content?.find((block) => block.type === "text" && typeof block.text === "string");
  if (!textBlock?.text) {
    throw new Error("SDK tool result did not include structuredContent or JSON text content.");
  }

  return JSON.parse(textBlock.text) as Record<string, unknown>;
}
