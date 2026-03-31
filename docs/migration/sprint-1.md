# Sprint 1 to Full `code.*` Migration Checkpoint

## Status

Sprint 1 established the npm-first TypeScript distribution layer and Rust engine boundary. Subsequent work migrated the full public `code.*` surface while keeping the contract stable.

### Migrated end-to-end in the current checkpoint

- `code.inspect_tree`
- `code.find_symbol`
- `code.list_endpoints`
- `code.search_text`
- `code.trace_symbol`
- `code.trace_callers`

## Runtime shape

```text
client
  -> TypeScript runtime (`packages/mcp-server`)
    -> public schema validation + response shaping
    -> capability request over stdio
      -> Rust engine (`crates/navigation-engine`)
        -> `workspace.inspect_tree`
        -> `workspace.find_symbol`
        -> `workspace.list_endpoints`
        -> `workspace.search_text`
        -> `workspace.trace_symbol`
        -> `workspace.trace_callers`
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

## Verification in the current checkpoint

Focused TypeScript runtime tests:

```bash
npm run mcp-server:test
```

Cross-runtime parity scaffold:

```bash
npm run contract-tests
```

`code.find_symbol` contract coverage no longer depends on Python. The Python oracle remains optional only for the legacy inspect-tree parity check.

`code.list_endpoints` now runs through the TS -> Rust path and is covered in the Rust engine suite for TypeScript, Java, Python, and Rust analyzer behavior.

`code.search_text`, `code.trace_symbol`, and `code.trace_callers` now also run through the TS -> Rust path, with dedicated TypeScript service tests and Rust capability coverage.

`code.trace_callers` uses AST/tree-sitter analysis in Rust for TypeScript and Java while preserving the public response contract. `implementationInterfaceChain` remains empty by design in the current engine behavior.

## Remaining thin follow-up work

1. Replace the interim JSON-over-stdio shell with verified MCP TypeScript SDK integration.
2. Keep extending parity and contract coverage on the migrated runtime path.
3. Decide whether richer interface/implementation reconstruction belongs in `trace_callers` beyond the current empty-chain behavior.

## Boundary rules frozen by the current checkpoint

- Public `code.*` names and envelopes stay stable.
- TypeScript owns public request validation and response shaping.
- Rust owns internal capability execution.
- TypeScript remains language-agnostic.
- All six public `code.*` tools are migrated through TS -> Rust.
- `code.find_symbol` now returns real Java, Python, Rust, and TypeScript-family definitions without requiring Python execution for acceptance.
- `code.list_endpoints` now returns React Router, Spring, Python web, and Rust web/graphql results through the Rust engine.
- `code.trace_callers` preserves the public contract but does not currently populate `implementationInterfaceChain`.
