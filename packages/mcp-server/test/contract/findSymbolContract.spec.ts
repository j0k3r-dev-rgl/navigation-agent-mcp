import test from "node:test";
import assert from "node:assert/strict";
import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import os from "node:os";
import path from "node:path";
import { promises as fs } from "node:fs";

const repoRoot = "/home/j0k3r/navigation-agent-mcp";

test("stdio runtime returns migrated find_symbol responses for Java and partial cases", async (t) => {
  const tempDir = await fs.mkdtemp(path.join(os.tmpdir(), "nav-mcp-find-symbol-"));
  const engineScriptPath = path.join(tempDir, "engine_stub.py");
  await fs.writeFile(engineScriptPath, buildEngineStubScript(), "utf8");

  const child = spawn(
    "node",
    [
      "--experimental-strip-types",
      "packages/mcp-server/src/bin/navigation-mcp.ts",
      "--transport",
      "stdio-legacy",
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

  await t.test("find_symbol preserves Java framework inference through the runtime", async () => {
    const response = await sendRequest(child, {
      id: 1,
      method: "call_tool",
      params: {
        name: "code.find_symbol",
        arguments: {
          symbol: "ExampleService",
          framework: "spring",
        },
      },
    });

    assert.equal(response.ok, true);
    assert.equal(response.result.tool, "code.find_symbol");
    assert.equal(response.result.status, "ok");
    assert.equal(response.result.summary, "Found 1 symbol definition for 'ExampleService'.");
    assert.equal(response.result.data.items[0].kind, "class");
    assert.equal(response.result.data.items[0].language, "java");
    assert.equal(response.result.meta.detection.effectiveLanguage, "java");
    assert.equal(response.result.meta.detection.framework, "spring");
  });

  await t.test("find_symbol preserves partial response envelopes through the runtime", async () => {
    const response = await sendRequest(child, {
      id: 2,
      method: "call_tool",
      params: {
        name: "code.find_symbol",
        arguments: {
          symbol: "load",
          match: "fuzzy",
          limit: 2,
        },
      },
    });

    assert.equal(response.ok, true);
    assert.equal(response.result.tool, "code.find_symbol");
    assert.equal(response.result.status, "partial");
    assert.equal(response.result.meta.truncated, true);
    assert.deepEqual(response.result.meta.counts, { returnedCount: 2, totalMatched: 3 });
    assert.equal(response.result.errors[0].code, "RESULT_TRUNCATED");
  });

  await t.test("find_symbol preserves the stable public envelope for python results", async () => {
    const response = await sendRequest(child, {
      id: 4,
      method: "call_tool",
      params: {
        name: "code.find_symbol",
        arguments: {
          symbol: "fetch_users",
          language: "python",
        },
      },
    });

    assert.equal(response.ok, true);
    assert.equal(response.result.tool, "code.find_symbol");
    assert.equal(response.result.status, "ok");
    assert.equal(response.result.summary, "Found 1 symbol definition for 'fetch_users'.");
    assert.deepEqual(response.result.data, {
      count: 1,
      returnedCount: 1,
      totalMatched: 1,
      items: [
        {
          symbol: "fetch_users",
          kind: "function",
          path: "app/users/service.py",
          line: 8,
          language: "python",
        },
      ],
    });
    assert.deepEqual(response.result.meta, {
      query: {
        symbol: "fetch_users",
        language: "python",
        framework: null,
        kind: "any",
        match: "exact",
        path: null,
        limit: 50,
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
    assert.deepEqual(response.result.errors, []);
  });

  await t.test("find_symbol preserves the stable public envelope for rust results", async () => {
    const response = await sendRequest(child, {
      id: 6,
      method: "call_tool",
      params: {
        name: "code.find_symbol",
        arguments: {
          symbol: "AnalyzerRegistry",
          language: "rust",
        },
      },
    });

    assert.equal(response.ok, true);
    assert.equal(response.result.tool, "code.find_symbol");
    assert.equal(response.result.status, "ok");
    assert.equal(response.result.summary, "Found 1 symbol definition for 'AnalyzerRegistry'.");
    assert.deepEqual(response.result.data, {
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
    assert.deepEqual(response.result.meta, {
      query: {
        symbol: "AnalyzerRegistry",
        language: "rust",
        framework: null,
        kind: "any",
        match: "exact",
        path: null,
        limit: 50,
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
    assert.deepEqual(response.result.errors, []);
  });

  await t.test("find_symbol preserves stable missing-path errors through the runtime", async () => {
    const response = await sendRequest(child, {
      id: 5,
      method: "call_tool",
      params: {
        name: "code.find_symbol",
        arguments: {
          symbol: "MissingController",
          path: "missing",
        },
      },
    });

    assert.equal(response.ok, true);
    assert.equal(response.result.tool, "code.find_symbol");
    assert.equal(response.result.status, "error");
    assert.equal(response.result.summary, "Path not found.");
    assert.equal(response.result.errors[0].code, "FILE_NOT_FOUND");
    assert.deepEqual(response.result.errors[0].details, { path: "missing" });
  });

  child.kill();
});

function buildEngineStubScript(): string {
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

    if payload.get("analyzerLanguage") == "java" and payload.get("publicLanguageFilter") == "java":
        response = {
            "id": request["id"],
            "ok": True,
            "result": {
                "resolvedPath": None,
                "items": [{
                    "symbol": "ExampleService",
                    "kind": "class",
                    "path": "src/main/java/demo/ExampleService.java",
                    "line": 10,
                    "language": "java"
                }],
                "totalMatched": 1,
                "truncated": False
            }
        }
        print(json.dumps(response), flush=True)
        continue

    if payload.get("analyzerLanguage") == "python" and payload.get("publicLanguageFilter") == "python":
        response = {
            "id": request["id"],
            "ok": True,
            "result": {
                "resolvedPath": None,
                "items": [{
                    "symbol": "fetch_users",
                    "kind": "function",
                    "path": "app/users/service.py",
                    "line": 8,
                    "language": "python"
                }],
                "totalMatched": 1,
                "truncated": False
            }
        }
        print(json.dumps(response), flush=True)
        continue

    if payload.get("analyzerLanguage") == "rust" and payload.get("publicLanguageFilter") == "rust":
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

    if payload.get("path") == "missing":
        response = {
            "id": request["id"],
            "ok": False,
            "error": {
                "code": "FILE_NOT_FOUND",
                "message": "Path 'missing' was not found inside the configured workspace root.",
                "retryable": False,
                "details": {"path": "missing"}
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
                {"symbol": "loader", "kind": "function", "path": "src/routes/a.ts", "line": 1, "language": "typescript"},
                {"symbol": "loadAction", "kind": "function", "path": "src/routes/b.ts", "line": 2, "language": "typescript"}
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
      reject(new Error(`navigation-mcp process exited before responding (code=${code ?? "null"}).`));
    };
    const onData = (chunk: Buffer | string) => {
      buffer += chunk.toString();
      const lines = buffer.split(/\r?\n/);
      buffer = lines.pop() ?? "";
      for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed) {
          continue;
        }
        cleanup();
        resolve(JSON.parse(trimmed));
        return;
      }
    };
    const cleanup = () => {
      child.stdout.off("data", onData);
      child.off("error", onError);
      child.off("exit", onExit);
    };

    child.stdout.on("data", onData);
    child.on("error", onError);
    child.on("exit", onExit);
  });
}
