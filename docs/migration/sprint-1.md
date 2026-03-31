# Sprint 1 and Sprint 2 Migration Checkpoint

## Status

Sprint 1 established the npm-first TypeScript distribution layer and Rust engine boundary. Sprint 2 extends that boundary to migrate `code.find_symbol` while keeping the public `code.*` contract stable.

### Migrated end-to-end after Sprint 2

- `code.inspect_tree`
- `code.find_symbol`

### Still on the compatibility path after Sprint 2

- `code.search_text`
- `code.trace_symbol`
- `code.trace_callers`
- `code.list_endpoints`

## Runtime shape

```text
client
  -> TypeScript runtime (`packages/mcp-server`)
    -> public schema validation + response shaping
    -> capability request over stdio
      -> Rust engine (`crates/navigation-engine`)
        -> `workspace.inspect_tree`
        -> `workspace.find_symbol`
```

The current TypeScript runtime uses a documented JSON-over-stdio loop as the safest migration checkpoint.
This remains intentional: the repository does not yet contain verified TypeScript MCP SDK integration evidence, so the runtime avoids guessing SDK APIs while the public contract stays stable.

## npm-first usage

Describe the public tool surface:

```bash
node --experimental-strip-types packages/mcp-server/src/bin/navigation-mcp.ts --describe-tools
```

Start the TypeScript runtime over stdio:

```bash
node --experimental-strip-types packages/mcp-server/src/bin/navigation-mcp.ts --transport stdio --workspace-root /path/to/workspace
```

Shortcut through npm workspaces:

```bash
npm run mcp-server:dev -- --workspace-root /path/to/workspace
```

## Environment notes

- The TypeScript CLI relies on Node 24 `--experimental-strip-types` in this repository state.
- The Rust engine client defaults to:

```bash
cargo run --quiet --manifest-path crates/navigation-engine/Cargo.toml
```

- Override the engine command when needed with `NAVIGATION_MCP_RUST_ENGINE_CMD` as a JSON string array.

Example:

```bash
export NAVIGATION_MCP_RUST_ENGINE_CMD='["cargo","run","--quiet","--manifest-path","crates/navigation-engine/Cargo.toml"]'
```

## Verification in Sprint 2

Focused TypeScript runtime tests:

```bash
npm run mcp-server:test
```

Cross-runtime parity scaffold:

```bash
npm run contract-tests
```

`code.find_symbol` contract coverage no longer depends on Python. The Python oracle remains optional only for the legacy inspect-tree parity check.

## Remaining thin follow-up work after Sprint 2

1. Replace the interim JSON-over-stdio shell with verified MCP TypeScript SDK integration.
2. Keep extending parity coverage before migrating the remaining `code.*` tools.
3. Document future Rust/Python analyzer roadmap separately from this Sprint 2 checkpoint.

## Boundary rules frozen by Sprint 2

- Public `code.*` names and envelopes stay stable.
- TypeScript owns public request validation and response shaping.
- Rust owns internal capability execution.
- TypeScript remains language-agnostic.
- `code.inspect_tree` and `code.find_symbol` are migrated through TS -> Rust.
- `code.find_symbol` now returns real Java and TypeScript-family definitions without requiring Python execution for acceptance.
