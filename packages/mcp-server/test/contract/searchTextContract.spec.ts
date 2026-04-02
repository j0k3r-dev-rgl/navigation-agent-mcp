import test from "node:test";
import assert from "node:assert/strict";
import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import os from "node:os";
import path from "node:path";
import { promises as fs } from "node:fs";

const repoRoot = "/home/j0k3r/navigation-agent-mcp";

test("stdio runtime returns migrated search_text responses through the Rust engine contract", async (t) => {
  const tempDir = await fs.mkdtemp(path.join(os.tmpdir(), "nav-mcp-search-text-"));
  const engineScriptPath = path.join(tempDir, "engine_stub.py");
  await fs.writeFile(engineScriptPath, buildEngineStubScript(), "utf8");

  const child = spawn(
    "npx",
    [
      "tsx",
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

  await t.test("search_text preserves the stable public envelope for partial results", async () => {
    const response = await sendRequest(child, {
      id: 1,
      method: "call_tool",
      params: {
        name: "code.search_text",
        arguments: {
          query: "loader",
          path: "src",
          limit: 1,
        },
      },
    });

    assert.equal(response.ok, true);
    assert.equal(response.result.tool, "code.search_text");
    assert.equal(response.result.status, "partial");
    assert.equal(
      response.result.summary,
      "Found 3 text matches across 2 files for 'loader' and returned a truncated subset.",
    );
    assert.deepEqual(response.result.data, {
      fileCount: 1,
      matchCount: 2,
      totalFileCount: 2,
      totalMatchCount: 3,
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
    assert.deepEqual(response.result.meta, {
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
        totalFileCount: 2,
        returnedMatchCount: 2,
        totalMatchCount: 3,
      },
      detection: {
        effectiveLanguage: null,
        framework: null,
      },
    });
    assert.equal(response.result.errors[0].code, "RESULT_TRUNCATED");
  });

  await t.test("search_text preserves language inference in the runtime request", async () => {
    const response = await sendRequest(child, {
      id: 2,
      method: "call_tool",
      params: {
        name: "code.search_text",
        arguments: {
          query: "ExampleService",
          framework: "spring",
        },
      },
    });

    assert.equal(response.ok, true);
    assert.equal(response.result.tool, "code.search_text");
    assert.equal(response.result.status, "ok");
    assert.equal(response.result.summary, "Found 1 text match in 1 file for 'ExampleService'.");
    assert.equal(response.result.meta.detection.effectiveLanguage, "java");
    assert.equal(response.result.meta.detection.framework, "spring");
    assert.equal(response.result.data.items[0].language, "java");
  });

  await t.test("search_text maps unsupported capability to backend execution failure", async () => {
    const response = await sendRequest(child, {
      id: 3,
      method: "call_tool",
      params: {
        name: "code.search_text",
        arguments: {
          query: "unsupported",
        },
      },
    });

    assert.equal(response.ok, true);
    assert.equal(response.result.status, "error");
    assert.equal(response.result.summary, "Text search failed.");
    assert.equal(response.result.errors[0].code, "BACKEND_EXECUTION_FAILED");
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

    if request["capability"] != "workspace.search_text":
        response = {"id": request["id"], "ok": False, "error": {"code": "UNSUPPORTED_CAPABILITY", "message": "Unsupported capability", "retryable": False, "details": {}}}
        print(json.dumps(response), flush=True)
        continue

    if payload.get("query") == "unsupported":
        response = {"id": request["id"], "ok": False, "error": {"code": "UNSUPPORTED_CAPABILITY", "message": "Capability 'workspace.search_text' is not implemented yet.", "retryable": False, "details": {"capability": "workspace.search_text"}}}
        print(json.dumps(response), flush=True)
        continue

    if payload.get("publicLanguageFilter") == "java":
        response = {
            "id": request["id"],
            "ok": True,
            "result": {
                "resolvedPath": None,
                "items": [{
                    "path": "src/main/java/demo/ExampleService.java",
                    "language": "java",
                    "matchCount": 1,
                    "matches": [{
                        "line": 12,
                        "text": "public class ExampleService {}",
                        "submatches": [{"start": 13, "end": 27, "text": "ExampleService"}],
                        "before": [],
                        "after": []
                    }]
                }],
                "totalFileCount": 1,
                "totalMatchCount": 1,
                "truncated": False
            }
        }
        print(json.dumps(response), flush=True)
        continue

    response = {
        "id": request["id"],
        "ok": True,
        "result": {
            "resolvedPath": "src",
            "items": [{
                "path": "src/routes/a.ts",
                "language": "typescript",
                "matchCount": 2,
                "matches": [
                    {
                        "line": 10,
                        "text": "export async function loader() {}",
                        "submatches": [{"start": 22, "end": 28, "text": "loader"}],
                        "before": [{"line": 9, "text": ""}],
                        "after": [{"line": 11, "text": "return null;"}]
                    },
                    {
                        "line": 20,
                        "text": "const loaderState = true;",
                        "submatches": [{"start": 6, "end": 12, "text": "loader"}],
                        "before": [],
                        "after": []
                    }
                ]
            }],
            "totalFileCount": 2,
            "totalMatchCount": 3,
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
        if (!line.trim()) {
          continue;
        }
        cleanup();
        resolve(JSON.parse(line));
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
