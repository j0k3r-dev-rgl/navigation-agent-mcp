import type { CodeToolName } from "../contracts/public/code.ts";
import type { ResponseEnvelope } from "../contracts/public/common.ts";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

export type NonMigratedCodeToolName = Exclude<
  CodeToolName,
  "code.inspect_tree" | "code.find_symbol" | "code.list_endpoints"
>;

export interface PythonFallbackBridge {
  readonly workspaceRoot: string;
  execute(
    toolName: NonMigratedCodeToolName,
    payload: Record<string, unknown>,
  ): Promise<ResponseEnvelope<unknown>>;
}

class PlaceholderPythonFallbackBridge implements PythonFallbackBridge {
  readonly workspaceRoot: string;

  constructor(workspaceRoot: string) {
    this.workspaceRoot = workspaceRoot;
  }

  async execute(
    toolName: NonMigratedCodeToolName,
    payload: Record<string, unknown>,
  ): Promise<ResponseEnvelope<unknown>> {
    const repoRoot = path.resolve(
      path.dirname(fileURLToPath(import.meta.url)),
      "../../../../",
    );
    const pythonExecutable = process.env.NAVIGATION_MCP_PYTHON ?? "python";

    const script = `
import asyncio
import json
import sys
from pathlib import Path

from navigation_mcp.app import create_mcp

tool_name = sys.argv[1]
workspace_root = Path(sys.argv[2])
payload = json.load(sys.stdin)

async def main():
    mcp = create_mcp(workspace_root=workspace_root)
    result = await mcp.call_tool(tool_name, payload)
    if isinstance(result, tuple) and len(result) == 2 and isinstance(result[1], dict):
        print(json.dumps(result[1]))
        return
    if isinstance(result, dict):
        print(json.dumps(result))
        return
    raise SystemExit(f"Unexpected MCP tool result shape: {type(result)!r}")

asyncio.run(main())
`;

    const child = spawn(
      pythonExecutable,
      ["-c", script, toolName, this.workspaceRoot],
      {
        cwd: repoRoot,
        env: {
          ...process.env,
          PYTHONPATH: path.join(repoRoot, "src"),
        },
        stdio: ["pipe", "pipe", "pipe"],
      },
    );

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
      throw new Error(
        `Python fallback for '${toolName}' failed with exit code ${exitCode ?? "null"}: ${stderr.trim()}`,
      );
    }

    return JSON.parse(stdout) as ResponseEnvelope<unknown>;
  }
}

export function createPythonFallbackBridge(options: {
  workspaceRoot: string;
}): PythonFallbackBridge {
  return new PlaceholderPythonFallbackBridge(options.workspaceRoot);
}
