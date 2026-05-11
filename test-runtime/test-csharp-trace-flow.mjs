#!/usr/bin/env node
// Manual runtime validation for C# code.trace_flow using examples/csharp

import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";
import { fileURLToPath } from "url";
import { dirname, join } from "path";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(__dirname, "..");
const workspaceRoot = join(repoRoot, "examples/csharp");

async function main() {
  console.log("=== C# trace_flow runtime validation ===\n");

  const client = new Client(
    { name: "navigation-agent-test", version: "1.0.0" },
    { capabilities: {} },
  );

  const transport = new StdioClientTransport({
    command: "npx",
    args: [
      "tsx",
      "packages/mcp-server/src/bin/navigation-mcp.ts",
      "--transport",
      "stdio",
      "--workspace-root",
      workspaceRoot,
    ],
    cwd: repoRoot,
    stderr: "inherit",
  });

  await client.connect(transport);

  try {
    const testCases = [
      {
        symbol: "OrderWorkflowService.ProcessOrderAsync",
        path: "src/Services/OrderWorkflowService.cs",
        expected: [
          "LoadDraftOrderAsync",
          "EnsureProcessable",
          "ApplyDiscount",
          "BuildPaymentRequest",
          "_paymentProcessor.ProcessPaymentAsync",
          "PersistPendingReviewAsync",
          "NotifyPendingReviewAsync",
          "PersistPaidOrderAsync",
          "NotifyPaidOrderAsync"
        ],
        notExpected: [
            "Console.WriteLine",
            "Math.Min",
            "ToString"
        ]
      }
    ];

    for (const testCase of testCases) {
      console.log(`--- code.trace_flow: ${testCase.symbol} in ${testCase.path} ---`);
      
      const result = await client.callTool({
        name: "code.trace_flow",
        arguments: {
          path: testCase.path,
          symbol: testCase.symbol,
          language: "csharp",
        },
      });

      const parsed = JSON.parse(result.content[0].text);
      
      if (parsed.status !== "ok") {
        console.error(`Error tracing ${testCase.symbol}:`, parsed.message || parsed.summary);
        process.exit(1);
      }

      const callees = parsed.data.root?.callers || [];
      console.log(`Found ${callees.length} callees.`);

      let missing = [];
      for (const expected of testCase.expected) {
        const found = callees.find(c => 
          (c.symbol && c.symbol.includes(expected))
        );
        if (found) {
          console.log(`✅ Found: ${expected}`);
        } else {
          console.error(`❌ Missing: ${expected}`);
          missing.push(expected);
        }
      }

      let foundNoise = [];
      for (const noise of testCase.notExpected) {
          const found = callees.find(c => 
            (c.symbol && c.symbol.includes(noise))
          );
          if (found) {
              console.error(`❌ Found noise: ${noise}`);
              foundNoise.push(noise);
          } else {
              console.log(`✅ Filtered: ${noise}`);
          }
      }

      if (missing.length > 0 || foundNoise.length > 0) {
        console.log("\nActual callees found:");
        callees.forEach((c, i) => {
          console.log(`${i + 1}. ${c.symbol} in ${c.path}`);
        });
        process.exit(1);
      }
      console.log("");
    }

    console.log("Validation PASSED!");

  } catch (error) {
    console.error("Validation FAILED with error:", error);
    process.exit(1);
  } finally {
    await transport.close();
  }
}

main().catch(error => {
  console.error(error);
  process.exit(1);
});
