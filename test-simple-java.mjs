#!/usr/bin/env node
// Simple test to verify Java analyzer works

import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";

const repoRoot = "/home/j0k3r/navigation-agent-mcp";
const workspaceRoot = "/home/j0k3r/sias/app/back";

async function main() {
  console.log("=== Simple Java trace_symbol test ===\n");

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
    // Test with a simple method that definitely has calls
    // Using editTitular which also calls another method
    console.log("--- Testing with 'editTitular' method ---");
    const result = await client.callTool({
      name: "code.trace_symbol",
      arguments: {
        path: "src/main/java/com/sistemasias/ar/modules/titular/infrastructure/web/rest/TitularRestController.java",
        symbol: "editTitular",
        analyzer_language: "java",
        max_depth: 2,
      },
    });
    const parsed = JSON.parse(result.content[0].text);
    console.log(`Status: ${parsed.status}`);
    console.log(`Callees count: ${parsed.data?.callees?.length || 0}`);
    console.log(`Items count: ${parsed.data?.items?.length || 0}`);
    
    if (parsed.data?.callees) {
      console.log("\nCallees found:");
      parsed.data.callees.forEach((callee, i) => {
        console.log(`  ${i + 1}. ${callee.callee} @ ${callee.path}:${callee.line} (depth: ${callee.depth})`);
      });
    }
    
    if (parsed.errors) {
      console.log("\nErrors:", parsed.errors);
    }

  } finally {
    await transport.close();
  }
}

main().catch(console.error);
