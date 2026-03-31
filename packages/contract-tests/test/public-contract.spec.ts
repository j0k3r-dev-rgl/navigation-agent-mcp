import test from "node:test";
import assert from "node:assert/strict";
import { spawn, spawnSync, type ChildProcessWithoutNullStreams } from "node:child_process";
import os from "node:os";
import path from "node:path";
import { promises as fs } from "node:fs";

const repoRoot = "/home/j0k3r/navigation-agent-mcp";

test("TS inspect_tree remains parity-compatible with the Python oracle when available", async (t) => {
  const oracleAvailability = spawnSync(
    "python",
    [
      "-c",
      [
        "import sys",
        "sys.path.insert(0, 'src')",
        "from navigation_mcp.app import create_mcp",
        "print('python-oracle-ok')",
      ].join("; "),
    ],
    { cwd: repoRoot, encoding: "utf8" },
  );

  if (oracleAvailability.status !== 0) {
    t.skip(`Python oracle unavailable in this environment: ${oracleAvailability.stderr.trim()}`);
    return;
  }

  const tempDir = await fs.mkdtemp(path.join(os.tmpdir(), "nav-mcp-contract-"));
  await fs.mkdir(path.join(tempDir, "src"), { recursive: true });
  await fs.mkdir(path.join(tempDir, ".hidden"), { recursive: true });
  await fs.mkdir(path.join(tempDir, ".git"), { recursive: true });
  await fs.writeFile(path.join(tempDir, "src", "main.py"), "print('ok')\n");
  await fs.writeFile(path.join(tempDir, ".hidden", "note.txt"), "secret\n");
  await fs.writeFile(path.join(tempDir, ".git", "config"), "[core]\n");

  const engineScriptPath = path.join(tempDir, "engine_stub.py");
  await fs.writeFile(engineScriptPath, buildEngineStubScript(), "utf8");

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

  const pythonResponse = await callPythonOracle(tempDir, {
    max_depth: 2,
    include_hidden: true,
  });
  const tsResponse = await sendRequest(tsChild, {
    id: 1,
    method: "call_tool",
    params: {
      name: "code.inspect_tree",
      arguments: {
        max_depth: 2,
        include_hidden: true,
      },
    },
  });

  tsChild.kill();

  assert.equal(tsResponse.ok, true);
  assert.equal(tsResponse.result.tool, pythonResponse.tool);
  assert.equal(tsResponse.result.status, pythonResponse.status);
  assert.equal(tsResponse.result.summary, pythonResponse.summary);
  assert.equal(tsResponse.result.data.root, pythonResponse.data.root);
  assert.equal(tsResponse.result.data.entryCount, pythonResponse.data.entryCount);
  assert.deepEqual(tsResponse.result.meta.resolvedPath, pythonResponse.meta.resolvedPath);
  assert.deepEqual(tsResponse.result.meta.truncated, pythonResponse.meta.truncated);
  assert.deepEqual(tsResponse.result.meta.counts, pythonResponse.meta.counts);
  assert.deepEqual(
    tsResponse.result.data.items.map((item: { path: string; name: string; type: string; depth: number; extension: string | null }) => ({
      path: item.path,
      name: item.name,
      type: item.type,
      depth: item.depth,
      extension: item.extension ?? null,
    })),
    pythonResponse.data.items.map((item: { path: string; name: string; type: string; depth: number; extension?: string | null }) => ({
      path: item.path,
      name: item.name,
      type: item.type,
      depth: item.depth,
      extension: item.extension ?? null,
    })),
  );
});

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

async function callPythonOracle(
  workspaceRoot: string,
  payload: Record<string, unknown>,
): Promise<any> {
  const script = [
    "import asyncio",
    "import json",
    "import sys",
    "from pathlib import Path",
    "sys.path.insert(0, 'src')",
    "from navigation_mcp.app import create_mcp",
    "async def main():",
    "    mcp = create_mcp(workspace_root=Path(sys.argv[1]))",
    "    payload = json.load(sys.stdin)",
    "    result = await mcp.call_tool('code.inspect_tree', payload)",
    "    if isinstance(result, tuple) and len(result) == 2 and isinstance(result[1], dict):",
    "        print(json.dumps(result[1]))",
    "        return",
    "    if isinstance(result, dict):",
    "        print(json.dumps(result))",
    "        return",
    "    raise SystemExit(f'Unexpected MCP tool result shape: {type(result)!r}')",
    "asyncio.run(main())",
  ].join("\n");

  const child = spawn("python", ["-c", script, workspaceRoot], {
    cwd: repoRoot,
    stdio: ["pipe", "pipe", "pipe"],
  });

  let stdout = "";
  let stderr = "";
  child.stdout.setEncoding("utf8");
  child.stderr.setEncoding("utf8");
  child.stdout.on("data", (chunk: string) => {
    stdout += chunk;
  });
  child.stderr.on("data", (chunk: string) => {
    stderr += chunk;
  });
  child.stdin.end(JSON.stringify(payload));

  const exitCode = await new Promise<number | null>((resolve, reject) => {
    child.on("error", reject);
    child.on("exit", (code) => resolve(code));
  });

  if (exitCode !== 0) {
    throw new Error(`Python oracle failed with exit code ${exitCode ?? "null"}: ${stderr.trim()}`);
  }

  return JSON.parse(stdout);
}

function buildEngineStubScript(): string {
  return `
import fnmatch
import json
import os
import sys
from datetime import UTC, datetime
from pathlib import Path

MAX_TREE_ITEMS = 2000
IGNORED = {".agent", ".agents", ".git", ".idea", "node_modules", ".react-router", ".vscode", ".claude", "build", "dist", ".next", "target", "coverage", ".turbo", ".cache", "tmp", "temp", "out"}

def should_ignore(name, include_hidden):
    if name in IGNORED:
        return True
    return not include_hidden and name.startswith('.')

def contains_hard_ignored(root, workspace_root):
    if root == workspace_root:
        return False
    return any(part in IGNORED for part in root.relative_to(workspace_root).parts)

def build_item(workspace_root, entry_path, depth, include_stats):
    stat = entry_path.lstat()
    item = {
        "path": entry_path.relative_to(workspace_root).as_posix(),
        "name": entry_path.name,
        "type": "directory" if entry_path.is_dir() else "file",
        "depth": depth,
        "extension": None if entry_path.is_dir() or not entry_path.suffix else entry_path.suffix.lower(),
        "stats": None,
    }
    if include_stats:
        item["stats"] = {
            "sizeBytes": int(stat.st_size),
            "modifiedAt": datetime.fromtimestamp(stat.st_mtime, tz=UTC).isoformat(),
            "isSymlink": entry_path.is_symlink(),
        }
    return item

for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    request = json.loads(line)
    payload = request["payload"]
    workspace_root = Path(request["workspaceRoot"]).resolve()
    relative_path = payload.get("path") or "."
    root_path = (workspace_root / relative_path).resolve()

    if not root_path.exists():
        response = {"id": request["id"], "ok": False, "error": {"code": "FILE_NOT_FOUND", "message": f"Path '{relative_path}' was not found inside the configured workspace root.", "retryable": False, "details": {"path": relative_path}}}
        print(json.dumps(response), flush=True)
        continue

    if workspace_root not in root_path.parents and root_path != workspace_root:
        response = {"id": request["id"], "ok": False, "error": {"code": "PATH_OUTSIDE_WORKSPACE", "message": f"Path '{relative_path}' is outside the configured workspace root.", "retryable": False, "details": {"path": relative_path}}}
        print(json.dumps(response), flush=True)
        continue

    root_public = "." if root_path == workspace_root else root_path.relative_to(workspace_root).as_posix()
    if contains_hard_ignored(root_path, workspace_root):
        response = {"id": request["id"], "ok": True, "result": {"root": root_public, "items": [], "truncated": False, "maxItems": MAX_TREE_ITEMS, "ignoredDirectories": sorted(IGNORED)}}
        print(json.dumps(response), flush=True)
        continue

    items = []
    truncated = False
    extensions = {value.lower() for value in payload.get("extensions") or []}
    file_pattern = payload.get("filePattern")

    def matches(entry_path):
        if entry_path.is_dir():
            return True
        if extensions and entry_path.suffix.lower() not in extensions:
            return False
        if file_pattern and not fnmatch.fnmatch(entry_path.name, file_pattern):
            return False
        return True

    def walk(current_path, current_depth):
        nonlocal truncated
        if truncated or current_depth >= payload.get("maxDepth", 3):
            return
        entries = sorted(os.scandir(current_path), key=lambda entry: (not entry.is_dir(follow_symlinks=False), entry.name.lower()))
        for entry in entries:
            if truncated:
                return
            if should_ignore(entry.name, payload.get("includeHidden", False)):
                continue
            entry_path = Path(entry.path)
            item_depth = len(entry_path.relative_to(root_path).parts)
            if matches(entry_path):
                items.append(build_item(workspace_root, entry_path, item_depth, payload.get("includeStats", False)))
                if len(items) >= MAX_TREE_ITEMS:
                    truncated = True
                    return
            if entry.is_dir(follow_symlinks=False) and item_depth < payload.get("maxDepth", 3) and not entry.is_symlink():
                walk(entry_path, current_depth + 1)

    if root_path.is_file():
        if matches(root_path):
            items.append(build_item(workspace_root, root_path, 1, payload.get("includeStats", False)))
    else:
        walk(root_path, 0)

    response = {"id": request["id"], "ok": True, "result": {"root": root_public, "items": items, "truncated": truncated, "maxItems": MAX_TREE_ITEMS, "ignoredDirectories": sorted(IGNORED)}}
    print(json.dumps(response), flush=True)
`;
}

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
