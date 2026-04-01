#!/usr/bin/env node
// Test trace_symbol recursivo con el workspace de Java

import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";

const repoRoot = "/home/j0k3r/navigation-agent-mcp";
const workspaceRoot = "/home/j0k3r/sias/app/back";

async function main() {
  console.log("=== trace_symbol recursive test ===\n");

  const client = new Client(
    { name: "navigation-agent-test", version: "1.0.0" },
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
    stderr: "pipe",
  });

  await client.connect(transport);

  try {
    // Primero, verificar que find_symbol funciona
    console.log("--- find_symbol: addMemberToTitular ---");
    const findResult = await client.callTool({
      name: "code.find_symbol",
      arguments: {
        path: "src/main/java/com/sistemasias/ar/modules/titular/infrastructure/web/rest/TitularRestController.java",
        symbol: "addMemberToTitular",
        analyzer_language: "java",
      },
    });
    console.log("find_symbol response:", JSON.parse(findResult.content[0].text));

    // Test trace_symbol para addMemberToTitular sin max_depth (original behavior)
    console.log("\n--- trace_symbol: addMemberToTitular (no max_depth) ---");
    const resultNoDepth = await client.callTool({
      name: "code.trace_symbol",
      arguments: {
        path: "src/main/java/com/sistemasias/ar/modules/titular/infrastructure/web/rest/TitularRestController.java",
        symbol: "addMemberToTitular",
        analyzer_language: "java",
      },
    });
    console.log("trace_symbol (no max_depth):", JSON.parse(resultNoDepth.content[0].text));

    // Test trace_symbol para addMemberToTitular con max_depth
    console.log("\n--- trace_symbol: addMemberToTitular (depth=3) ---");
    const result = await client.callTool({
      name: "code.trace_symbol",
      arguments: {
        path: "src/main/java/com/sistemasias/ar/modules/titular/infrastructure/web/rest/TitularRestController.java",
        symbol: "addMemberToTitular",
        analyzer_language: "java",
        max_depth: 3,
      },
    });
    const parsed = JSON.parse(result.content[0].text);
    console.log(`Status: ${parsed.status}`);
    console.log(`Resolved path: ${parsed.data.resolvedPath}`);
    console.log(`Total callees found: ${parsed.data.totalMatched}`);
    console.log(`Files involved: ${parsed.data.items?.length || 0}`);
    console.log("\nCallees:");
    if (parsed.data.callees) {
      parsed.data.callees.forEach((callee, i) => {
        console.log(`  ${i + 1}. ${callee.callee} @ ${callee.path}:${callee.line}-${callee.end_line} (depth: ${callee.depth})`);
        if (callee.snippet) {
          console.log(`     snippet: ${callee.snippet.substring(0, 80)}...`);
        }
      });
    }

  } finally {
    await transport.close();
  }
}

main().catch(console.error);
