import test from "node:test";
import assert from "node:assert/strict";
import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import os from "node:os";
import path from "node:path";
import { promises as fs } from "node:fs";

const repoRoot = "/home/j0k3r/navigation-agent-mcp";

test("stdio runtime lists tools and returns migrated inspect_tree responses", async (t) => {
  const tempDir = await fs.mkdtemp(path.join(os.tmpdir(), "nav-mcp-ts-"));
  await fs.mkdir(path.join(tempDir, "src"), { recursive: true });
  await fs.mkdir(path.join(tempDir, ".hidden"), { recursive: true });
  await fs.mkdir(path.join(tempDir, ".git"), { recursive: true });
  await fs.mkdir(path.join(tempDir, "node_modules"), { recursive: true });
  await fs.writeFile(path.join(tempDir, "src", "main.py"), "print('ok')\n");
  await fs.writeFile(path.join(tempDir, ".hidden", "note.txt"), "secret\n");
  await fs.writeFile(path.join(tempDir, ".git", "config"), "[core]\n");
  await fs.writeFile(path.join(tempDir, "node_modules", "pkg.js"), "module.exports = {}\n");

  const engineScriptPath = path.join(tempDir, "engine_stub.py");
  await fs.writeFile(engineScriptPath, buildEngineStubScript(), "utf8");

  const child = spawn(
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

  await t.test("list_tools preserves the stable public tool set", async () => {
    const response = await sendRequest(child, { id: 1, method: "list_tools" });
    assert.equal(response.ok, true);
    assert.deepEqual(
      new Set(response.result.map((tool: { name: string }) => tool.name)),
      new Set([
        "code.find_symbol",
        "code.search_text",
        "code.trace_symbol",
        "code.trace_callers",
        "code.list_endpoints",
        "code.inspect_tree",
      ]),
    );
  });

  await t.test("inspect_tree preserves include_hidden and hard-ignore behavior", async () => {
    const response = await sendRequest(child, {
      id: 2,
      method: "call_tool",
      params: {
        name: "code.inspect_tree",
        arguments: { max_depth: 2, include_hidden: true },
      },
    });

    assert.equal(response.ok, true);
    assert.equal(response.result.status, "ok");
    assert.equal(response.result.tool, "code.inspect_tree");
    assert.equal(response.result.data.root, ".");
    assert.equal(response.result.meta.resolvedPath, ".");
    const paths = new Set(response.result.data.items.map((item: { path: string }) => item.path));
    assert.ok(paths.has(".hidden"));
    assert.ok(paths.has(".hidden/note.txt"));
    assert.ok(paths.has("src"));
    assert.ok(paths.has("src/main.py"));
    assert.ok(![...paths].some((value) => value === ".git" || value.startsWith(".git/")));
    assert.ok(![...paths].some((value) => value === "node_modules" || value.startsWith("node_modules/")));
  });

  await t.test("inspect_tree preserves missing-path public error mapping", async () => {
    const response = await sendRequest(child, {
      id: 3,
      method: "call_tool",
      params: {
        name: "code.inspect_tree",
        arguments: { path: "missing" },
      },
    });

    assert.equal(response.ok, true);
    assert.equal(response.result.status, "error");
    assert.equal(response.result.summary, "Path not found.");
    assert.equal(response.result.errors[0].code, "FILE_NOT_FOUND");
    assert.deepEqual(response.result.errors[0].details, { path: "missing" });
  });

  await t.test("inspect_tree preserves truncation as a stable partial response", async () => {
    await fs.mkdir(path.join(tempDir, "many"), { recursive: true });
    await Promise.all(
      Array.from({ length: 2001 }, (_, index) =>
        fs.writeFile(path.join(tempDir, "many", `file_${String(index).padStart(4, "0")}.txt`), "x\n"),
      ),
    );

    const response = await sendRequest(child, {
      id: 4,
      method: "call_tool",
      params: {
        name: "code.inspect_tree",
        arguments: { path: "many", max_depth: 1 },
      },
    });

    assert.equal(response.ok, true);
    assert.equal(response.result.status, "partial");
    assert.equal(response.result.meta.truncated, true);
    assert.equal(response.result.data.entryCount, 2000);
    assert.equal(response.result.meta.counts.returnedCount, 2000);
    assert.equal(response.result.meta.counts.totalMatched, null);
    assert.equal(response.result.errors[0].code, "RESULT_TRUNCATED");
  });

  child.kill();
});

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

    if request["capability"] != "workspace.inspect_tree":
        response = {"id": request["id"], "ok": False, "error": {"code": "UNSUPPORTED_CAPABILITY", "message": "Unsupported capability", "retryable": False, "details": {}}}
        print(json.dumps(response), flush=True)
        continue

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
    truncated = {"value": False}
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

    if root_path.is_file():
        if matches(root_path):
            items.append(build_item(workspace_root, root_path, 1, payload.get("includeStats", False)))
    else:
        def walk(current_path, current_depth):
            if truncated["value"] or current_depth >= payload.get("maxDepth", 3):
                return
            entries = sorted(os.scandir(current_path), key=lambda entry: (not entry.is_dir(follow_symlinks=False), entry.name.lower()))
            for entry in entries:
                if truncated["value"]:
                    return
                if should_ignore(entry.name, payload.get("includeHidden", False)):
                    continue
                entry_path = Path(entry.path)
                item_depth = len(entry_path.relative_to(root_path).parts)
                if matches(entry_path):
                    items.append(build_item(workspace_root, entry_path, item_depth, payload.get("includeStats", False)))
                    if len(items) >= MAX_TREE_ITEMS:
                        truncated["value"] = True
                        return
                if entry.is_dir(follow_symlinks=False) and item_depth < payload.get("maxDepth", 3) and not entry.is_symlink():
                    walk(entry_path, current_depth + 1)
        walk(root_path, 0)

    response = {"id": request["id"], "ok": True, "result": {"root": root_public, "items": items, "truncated": truncated["value"], "maxItems": MAX_TREE_ITEMS, "ignoredDirectories": sorted(IGNORED)}}
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