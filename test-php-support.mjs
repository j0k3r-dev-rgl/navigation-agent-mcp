#!/usr/bin/env node

import { spawn } from "child_process";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const enginePath = path.join(__dirname, "crates/navigation-engine");

async function sendRequest(method, params) {
  return new Promise((resolve, reject) => {
    const child = spawn("cargo", ["run", "--bin", "navigation-engine-cli", "--", method, JSON.stringify(params)], {
      cwd: enginePath,
      stdio: ["pipe", "pipe", "pipe"],
    });

    let stdout = "";
    let stderr = "";

    child.stdout.on("data", (data) => {
      stdout += data.toString();
    });

    child.stderr.on("data", (data) => {
      stderr += data.toString();
    });

    child.on("close", (code) => {
      if (code !== 0) {
        reject(new Error(`Process exited with code ${code}\nStderr: ${stderr}`));
      } else {
        try {
          resolve(JSON.parse(stdout));
        } catch (e) {
          reject(new Error(`Failed to parse JSON: ${stdout}`));
        }
      }
    });
  });
}

async function testPhpSupport() {
  console.log("🧪 Testing PHP support in navigation-agent-mcp\n");

  const phpExampleDir = path.join(__dirname, "examples/php");

  // Test 1: find_symbol
  console.log("1️⃣  Testing code.find_symbol for PHP classes...");
  try {
    const result = await sendRequest("find_symbol", {
      workspace_root: phpExampleDir,
      analyzer_language: "php",
      symbol: "UserService",
      kind: "any",
      match_mode: "exact",
      limit: 10,
    });
    
    if (result.symbols && result.symbols.length > 0) {
      console.log(`   ✅ Found ${result.symbols.length} symbol(s)`);
      console.log(`      - ${result.symbols[0].symbol} (${result.symbols[0].kind}) at ${result.symbols[0].path}:${result.symbols[0].line}`);
    } else {
      console.log("   ❌ No symbols found");
    }
  } catch (error) {
    console.log(`   ❌ Error: ${error.message}`);
  }

  // Test 2: trace_flow
  console.log("\n2️⃣  Testing code.trace_flow for PHP methods...");
  try {
    const servicePath = path.join(phpExampleDir, "src/Service/UserService.php");
    const result = await sendRequest("trace_flow", {
      workspace_root: phpExampleDir,
      analyzer_language: "php",
      path: servicePath,
      symbol: "createUser",
    });

    if (result.callees && result.callees.length > 0) {
      console.log(`   ✅ Found ${result.callees.length} callee(s)`);
      result.callees.slice(0, 3).forEach((callee) => {
        console.log(`      - ${callee.callee} at line ${callee.line}`);
      });
    } else {
      console.log("   ⚠️  No callees found (might be expected)");
    }
  } catch (error) {
    console.log(`   ❌ Error: ${error.message}`);
  }

  // Test 3: trace_callers
  console.log("\n3️⃣  Testing code.trace_callers for PHP methods...");
  try {
    const repoPath = path.join(phpExampleDir, "src/Repository/MemoryUserRepository.php");
    const result = await sendRequest("trace_callers", {
      workspace_root: phpExampleDir,
      analyzer_language: "php",
      path: repoPath,
      symbol: "save",
      recursive: false,
    });

    if (result.callers && result.callers.length > 0) {
      console.log(`   ✅ Found ${result.callers.length} caller(s)`);
      result.callers.slice(0, 3).forEach((caller) => {
        console.log(`      - ${caller.caller} calls save at ${caller.path}:${caller.line}`);
      });
    } else {
      console.log("   ⚠️  No callers found");
    }
  } catch (error) {
    console.log(`   ❌ Error: ${error.message}`);
  }

  console.log("\n✨ PHP support test completed!");
}

testPhpSupport().catch(console.error);
