import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { once } from "node:events";
import { EOL } from "node:os";
const _require = createRequire(import.meta.url);
export class RustEngineClient {
    #options;
    #child = null;
    #pending = new Map();
    #buffer = "";
    constructor(options = {}) {
        this.#options = options;
    }
    async request(request) {
        const child = this.#ensureChild();
        const responsePromise = new Promise((resolve, reject) => {
            this.#pending.set(request.id, {
                resolve: resolve,
                reject,
            });
        });
        child.stdin.write(`${JSON.stringify(request)}${EOL}`);
        return responsePromise;
    }
    async close() {
        if (!this.#child) {
            return;
        }
        const child = this.#child;
        this.#child = null;
        child.stdin.end();
        child.kill();
        await once(child, "exit").catch(() => undefined);
    }
    #ensureChild() {
        if (this.#child) {
            return this.#child;
        }
        const command = resolveEngineCommand(this.#options.command);
        const [executable, ...args] = command;
        const child = spawn(executable, args, {
            cwd: this.#options.cwd ?? process.cwd(),
            env: { ...process.env, ...this.#options.env },
            stdio: ["pipe", "pipe", "pipe"],
        });
        child.stdout.setEncoding("utf8");
        child.stdout.on("data", (chunk) => {
            this.#buffer += chunk;
            const lines = this.#buffer.split(/\r?\n/);
            this.#buffer = lines.pop() ?? "";
            for (const line of lines) {
                const trimmed = line.trim();
                if (!trimmed) {
                    continue;
                }
                let parsed;
                try {
                    parsed = JSON.parse(trimmed);
                }
                catch (error) {
                    this.#failAll(new Error(`Rust engine returned invalid JSON: ${error.message}`));
                    continue;
                }
                const pending = this.#pending.get(parsed.id);
                if (!pending) {
                    continue;
                }
                this.#pending.delete(parsed.id);
                pending.resolve(parsed);
            }
        });
        child.stderr.setEncoding("utf8");
        child.stderr.on("data", () => {
            // keep stderr attached for debugging without leaking into public protocol handling
        });
        child.on("error", (error) => {
            this.#failAll(error instanceof Error ? error : new Error(String(error)));
            this.#child = null;
        });
        child.on("exit", (code, signal) => {
            this.#failAll(new Error(`Rust engine exited unexpectedly (code=${code ?? "null"}, signal=${signal ?? "null"}).`));
            this.#child = null;
        });
        this.#child = child;
        return child;
    }
    #failAll(error) {
        for (const pending of this.#pending.values()) {
            pending.reject(error);
        }
        this.#pending.clear();
    }
}
function resolveEngineCommand(override) {
    if (override && override.length > 0) {
        return override;
    }
    const configured = process.env.NAVIGATION_MCP_RUST_ENGINE_CMD;
    if (configured) {
        const parsed = JSON.parse(configured);
        if (Array.isArray(parsed) && parsed.every((value) => typeof value === "string")) {
            return parsed;
        }
        throw new Error("NAVIGATION_MCP_RUST_ENGINE_CMD must be a JSON array of strings.");
    }
    // Try to find the pre-compiled binary bundled in the platform-specific optional package.
    const binaryName = process.platform === "win32" ? "navigation-engine.exe" : "navigation-engine";
    const platformPackages = {
        "linux-x64": "@navigation-agent/mcp-server-linux-x64",
        "linux-arm64": "@navigation-agent/mcp-server-linux-arm64",
        "darwin-x64": "@navigation-agent/mcp-server-darwin-x64",
        "darwin-arm64": "@navigation-agent/mcp-server-darwin-arm64",
        "win32-x64": "@navigation-agent/mcp-server-win32-x64",
    };
    const pkgName = platformPackages[`${process.platform}-${process.arch}`];
    if (pkgName) {
        try {
            const pkgJsonPath = _require.resolve(`${pkgName}/package.json`);
            const binaryPath = join(dirname(pkgJsonPath), binaryName);
            if (existsSync(binaryPath)) {
                return [binaryPath];
            }
        }
        catch {
            // Optional package not installed — fall through to dev fallback.
        }
    }
    // Dev fallback: build and run via cargo.
    return [
        "cargo",
        "run",
        "--quiet",
        "--manifest-path",
        "crates/navigation-engine/Cargo.toml",
    ];
}
