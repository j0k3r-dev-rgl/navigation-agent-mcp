#!/usr/bin/env node
import { createMcpServer } from "../app/createMcpServer.js";
function getFlagValue(argv, flag) {
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
    process.stdout.write(`${JSON.stringify({
        name: server.name,
        version: server.version,
        workspaceRoot: server.workspaceRoot,
        toolCount: server.tools.length,
        tools: server.listTools().map(({ execute, sdkInputSchema, ...tool }) => tool),
        runtime: "navigation-sdk-stdio",
        transports: ["stdio", "stdio-legacy"],
    }, null, 2)}\n`);
}
else if ((getFlagValue(argv, "--transport") ?? "stdio") === "stdio") {
    await server.serveStdio();
}
else if (getFlagValue(argv, "--transport") === "stdio-legacy") {
    await server.serveStdioLegacy();
}
else {
    process.stderr.write("Supported transports: stdio, stdio-legacy.\n");
    process.exitCode = 1;
}
