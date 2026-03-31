import test from "node:test";
import assert from "node:assert/strict";
import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import os from "node:os";
import path from "node:path";
import { promises as fs } from "node:fs";

const repoRoot = "/home/j0k3r/navigation-agent-mcp";

test("TS find_symbol preserves the public contract while adding Python and Rust", async () => {
  const tempDir = await fs.mkdtemp(path.join(os.tmpdir(), "nav-mcp-find-symbol-contract-"));
  const engineScriptPath = path.join(tempDir, "engine_find_symbol_stub.py");
  await fs.writeFile(engineScriptPath, buildFindSymbolEngineStubScript(), "utf8");

  const tsChild = spawn(
    "node",
    [
      "--experimental-strip-types",
      "packages/mcp-server/src/bin/navigation-mcp.ts",
      "--transport",
      "stdio",
      "--workspace-root",
      tempDir,
    ],
    {
      cwd: repoRoot,
      env: {
        ...process.env,
        NAVIGATION_MCP_RUST_ENGINE_CMD: JSON.stringify(["python", engineScriptPath]),
      },
      stdio: ["pipe", "pipe", "pipe"],
    },
  );

  const exactResponse = await sendRequest(tsChild, {
    id: 2,
    method: "call_tool",
    params: {
      name: "code.find_symbol",
      arguments: {
        symbol: "UserRestController",
        language: "java",
        match: "exact",
        limit: 5,
      },
    },
  });

  assert.equal(exactResponse.ok, true);
  assert.equal(exactResponse.result.tool, "code.find_symbol");
  assert.equal(exactResponse.result.status, "ok");
  assert.equal(
    exactResponse.result.summary,
    "Found 1 symbol definition for 'UserRestController'.",
  );
  assert.deepEqual(exactResponse.result.data, {
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
  assert.deepEqual(exactResponse.result.meta, {
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

  const partialResponse = await sendRequest(tsChild, {
    id: 3,
    method: "call_tool",
    params: {
      name: "code.find_symbol",
      arguments: {
        symbol: "Titular",
        language: "java",
        kind: "class",
        match: "fuzzy",
        limit: 2,
      },
    },
  });

  assert.equal(partialResponse.ok, true);
  assert.equal(partialResponse.result.tool, "code.find_symbol");
  assert.equal(partialResponse.result.status, "partial");
  assert.equal(
    partialResponse.result.summary,
    "Found 3 symbol definitions for 'Titular' and returned a truncated subset.",
  );
  assert.equal(partialResponse.result.meta.truncated, true);
  assert.deepEqual(partialResponse.result.meta.counts, {
    returnedCount: 2,
    totalMatched: 3,
  });
  assert.equal(partialResponse.result.errors[0].code, "RESULT_TRUNCATED");
  assert.deepEqual(
    partialResponse.result.data.items.map(
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

  const pythonResponse = await sendRequest(tsChild, {
    id: 4,
    method: "call_tool",
    params: {
      name: "code.find_symbol",
      arguments: {
        symbol: "fetch_users",
        language: "python",
        match: "exact",
        limit: 5,
      },
    },
  });

  const rustResponse = await sendRequest(tsChild, {
    id: 5,
    method: "call_tool",
    params: {
      name: "code.find_symbol",
      arguments: {
        symbol: "AnalyzerRegistry",
        language: "rust",
        match: "exact",
        limit: 5,
      },
    },
  });

  tsChild.kill();

  assert.equal(pythonResponse.ok, true);
  assert.equal(pythonResponse.result.tool, "code.find_symbol");
  assert.equal(pythonResponse.result.status, "ok");
  assert.equal(
    pythonResponse.result.summary,
    "Found 1 symbol definition for 'fetch_users'.",
  );
  assert.deepEqual(pythonResponse.result.data, {
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
  assert.deepEqual(pythonResponse.result.meta, {
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
  assert.deepEqual(pythonResponse.result.errors, []);

  assert.equal(rustResponse.ok, true);
  assert.equal(rustResponse.result.tool, "code.find_symbol");
  assert.equal(rustResponse.result.status, "ok");
  assert.equal(
    rustResponse.result.summary,
    "Found 1 symbol definition for 'AnalyzerRegistry'.",
  );
  assert.deepEqual(rustResponse.result.data, {
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
  assert.deepEqual(rustResponse.result.meta, {
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
  assert.deepEqual(rustResponse.result.errors, []);
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

async function sendRequest(
  child: ChildProcessWithoutNullStreams,
  request: Record<string, unknown>,
): Promise<any> {
  child.stdin.write(`${JSON.stringify(request)}\n`);

  return new Promise((resolve, reject) => {
    let buffer = "";
    const onError = (error: Error) => {
      cleanup();
      reject(error);
    };
    const onExit = (code: number | null) => {
      cleanup();
      reject(new Error(`TypeScript runtime exited before responding (code=${code ?? "null"}).`));
    };
    const onData = (chunk: Buffer | string) => {
      buffer += chunk.toString();
      const newlineIndex = buffer.indexOf("\n");
      if (newlineIndex === -1) {
        return;
      }
      const line = buffer.slice(0, newlineIndex);
      cleanup();
      resolve(JSON.parse(line));
    };
    const cleanup = () => {
      child.stdout.off("data", onData);
      child.off("error", onError);
      child.off("exit", onExit);
    };

    child.stdout.on("data", onData);
    child.once("error", onError);
    child.once("exit", onExit);
  });
}