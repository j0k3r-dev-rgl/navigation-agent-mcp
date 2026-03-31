#!/usr/bin/env -S node --experimental-strip-types

import { createMcpServer } from "../app/createMcpServer.ts";

function getFlagValue(argv: readonly string[], flag: string): string | null {
  const flagIndex = argv.findIndex((value) => value === flag);
  if (flagIndex === -1) {
    return null;
  }
  const value = argv[flagIndex + 1];
  return value && !value.startsWith("--") ? value : null;
}

const argv = process.argv.slice(2);
const server = createMcpServer({
  workspaceRoot: getFlagValue(argv, "--workspace-root") ?? process.cwd(),
});

if (argv.includes("--describe-tools")) {
  process.stdout.write(
    `${JSON.stringify(
      {
        name: server.name,
        workspaceRoot: server.workspaceRoot,
        toolCount: server.tools.length,
        tools: server.listTools().map(({ execute, ...tool }) => tool),
        runtime: "navigation-json-stdio",
      },
      null,
      2,
    )}\n`,
  );
} else if ((getFlagValue(argv, "--transport") ?? "stdio") === "stdio") {
  await server.serveStdio();
} else {
  process.stderr.write(
    "Only --transport stdio is implemented in the TypeScript runtime for Sprint 1.\n",
  );
  process.exitCode = 1;
}
