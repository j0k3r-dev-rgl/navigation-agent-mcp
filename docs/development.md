# Development

## Tech stack

- TypeScript MCP runtime (`packages/mcp-server`)
- Rust analysis engine with tree-sitter (`crates/navigation-engine`)
- npm workspace at the repo root

## Project layout

```text
packages/
├── mcp-server/         # TypeScript runtime shell and public contract
└── contract-tests/     # runtime/contract guards
crates/
└── navigation-engine/  # Rust capability engine and AST debug binaries
```

## Run locally

```bash
npm install
npm run mcp-server:dev -- --workspace-root /path/to/workspace
```

### Describe the public tool surface

Use `tsx`, not `node --experimental-strip-types`:

```bash
npx tsx packages/mcp-server/src/bin/navigation-mcp.ts --describe-tools
```

That prints the actual public `code.*` tool surface, schemas, and transports.

## Tests

```bash
cargo test --manifest-path crates/navigation-engine/Cargo.toml
npm run mcp-server:test
```

## Useful real-workspace smoke tests

### Java / Spring

```bash
npx tsx packages/mcp-server/src/bin/navigation-mcp.ts --transport stdio-legacy --workspace-root /home/j0k3r/sias/app/back
```

Good real cases:

- `code.list_endpoints` with `framework: "spring"`
- `code.trace_flow` on `RootUserGraphQLController#getUsersByDependency`
- `code.trace_callers` on `RootGetUserUseCase#getUsers`

### TypeScript / React Router

```bash
npx tsx packages/mcp-server/src/bin/navigation-mcp.ts --transport stdio-legacy --workspace-root /home/j0k3r/sias/app/front
```

Good real cases:

- `code.list_endpoints` with `framework: "react-router"`
- `code.trace_flow` on `app/routes/change-password.tsx#action`
- `code.trace_callers` on `getRoleRoute`

### Rust

```bash
npx tsx packages/mcp-server/src/bin/navigation-mcp.ts --transport stdio-legacy --workspace-root /home/j0k3r/navigation-agent-mcp
```

Good real cases:

- `code.find_symbol` on `AnalyzerRegistry`
- `code.trace_flow` on `crates/navigation-engine/src/capabilities/trace_flow.rs#build`

Current caveat verified in a real case:

- `code.trace_callers` for Rust is publicly exposed but the tested real symbols still returned no callers even when callers exist in code

### Go example

```bash
npx tsx packages/mcp-server/src/bin/navigation-mcp.ts --transport stdio-legacy --workspace-root /home/j0k3r/navigation-agent-mcp/examples/go
```

Use this only as an internal WIP validation target for now.

Current real behavior:

- `find_symbol` misses known Go symbols
- `trace_flow` returns empty callees on real handlers
- `trace_callers` fails on known symbols

Do not document Go as publicly supported until the TS contract and runtime behavior are aligned.

## Rust engine override

The TypeScript runtime starts the Rust engine via `cargo run` by default. Override the command with:

```bash
export NAVIGATION_MCP_RUST_ENGINE_CMD='["./path/to/navigation-engine"]'
```

## AST/debug binaries

Internal AST/debug helpers live in:

```text
crates/navigation-engine/src/bin/
```

Examples:

- `inspect_rust_ast.rs`
- `inspect_go_ast.rs`
- `inspect_go_callees.rs`
- `inspect_ts_ast.rs`
- `inspect_ts_callees.rs`

These are development aids for analyzer work. They are not part of the public MCP tool surface.

## Environment variables

| Variable | Description |
|---|---|
| `NAVIGATION_MCP_WORKSPACE_ROOT` | Workspace root to analyze. Defaults to CWD. |
| `NAVIGATION_MCP_RUST_ENGINE_CMD` | JSON string array to override the Rust engine command. |
