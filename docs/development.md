# Development

## Tech stack

- TypeScript runtime (npm-first)
- Rust engine (tree-sitter AST analysis)

## Project layout

```text
packages/
├── mcp-server/         # TypeScript runtime shell
└── contract-tests/     # cross-runtime parity guards
crates/
└── navigation-engine/  # Rust capability engine
```

## Run locally

```bash
npm install
npm run mcp-server:dev -- --workspace-root /path/to/workspace
```

Or describe the tool surface without starting the full server:

```bash
node --experimental-strip-types packages/mcp-server/src/bin/navigation-mcp.ts --describe-tools
```

## Tests

```bash
cargo test --manifest-path crates/navigation-engine/Cargo.toml
npm run mcp-server:test
```

## Rust engine override

The TypeScript runtime starts the Rust engine via `cargo run` by default. Override the command with:

```bash
export NAVIGATION_MCP_RUST_ENGINE_CMD='["./path/to/navigation-engine"]'
```

## Environment variables

| Variable | Description |
|---|---|
| `NAVIGATION_MCP_WORKSPACE_ROOT` | Workspace root to analyze. Defaults to CWD. |
| `NAVIGATION_MCP_RUST_ENGINE_CMD` | JSON string array to override the Rust engine command. |
