# Testing Guide

## Goal

The test suite protects the **public V1 contract**, not internal implementation trivia.

That means the highest-value tests focus on:

- MCP tool discoverability and schema shape
- normalized envelopes and stable metadata
- `find_symbol` exact/fuzzy/language-framework semantics
- `list_endpoints` route/rest/graphql discovery semantics
- path safety errors
- inspect-tree hard-ignore behavior
- truncation semantics for bounded responses

## Current test layout

```text
tests/
├── conftest.py
├── test_inspect_tree_contract.py
├── test_search_text_service.py
└── test_tool_registration.py

packages/mcp-server/test/
├── contract/
│   ├── findSymbolContract.spec.ts
│   └── inspectTreeContract.spec.ts
└── unit/
    ├── findSymbolService.spec.ts
    ├── normalizeFindSymbolInput.spec.ts
    └── toolRegistration.spec.ts

packages/contract-tests/test/
└── contract/
    ├── findSymbolContract.spec.ts
    └── inspectTreeParity.spec.ts

crates/navigation-engine/tests/
├── analyzers_java.rs
├── analyzers_python.rs
├── analyzers_rust.rs
├── analyzers_types.rs
├── analyzers_typescript.rs
├── capabilities_find_symbol.rs
├── capabilities_inspect_tree.rs
├── capabilities_list_endpoints.rs
├── protocol.rs
└── workspace.rs
```

## What each test module covers

### `test_tool_registration.py`

- verifies the six V1 tools are registered
- verifies key schema constraints that clients discover through MCP

### `test_inspect_tree_contract.py`

- verifies hidden-vs-hard-ignored behavior
- verifies scoped inspection into hard-ignored directories returns an empty safe result
- verifies truncation at the tree safety cap
- verifies normalized path errors at the public tool layer

### `test_search_text_service.py`

- verifies service-level normalization for truncated search responses
- verifies count metadata and effective language detection

### `packages/mcp-server/test/unit/findSymbolService.spec.ts`

- verifies migrated request shaping for `react-router`, `javascript`, and `spring`
- verifies stable summary, truncation, and error-envelope mapping at the TS service layer

### `packages/mcp-server/test/contract/findSymbolContract.spec.ts`

- verifies the migrated `code.find_symbol` stdio runtime path without Python
- verifies Java framework inference and partial-result envelopes at the public runtime boundary

### `packages/mcp-server/test/unit/toolRegistration.spec.ts`

- verifies the npm-first TypeScript runtime exposes the same six `code.*` tools
- verifies the discoverable schema defaults/required fields remain stable

### `packages/mcp-server/test/contract/inspectTreeContract.spec.ts`

- verifies the migrated `code.inspect_tree` path through the TS runtime
- verifies hidden-vs-hard-ignored behavior, truncation, and missing-path errors

### `packages/contract-tests/test/contract/findSymbolContract.spec.ts`

- verifies the public `code.find_symbol` contract through the TypeScript runtime without requiring Python

### `packages/contract-tests/test/contract/inspectTreeParity.spec.ts`

- compares the TypeScript `code.inspect_tree` output with the Python oracle when that oracle is available
- skips with the exact blocking reason when Python MCP dependencies are missing in the environment

### `crates/navigation-engine/tests/analyzers_*.rs`

- verifies migrated analyzer behavior directly in Rust
- covers TypeScript, Java, Python, and Rust symbol/endpoint extraction

### `crates/navigation-engine/tests/capabilities_*.rs`

- verifies Rust capability handlers for `inspect_tree`, `find_symbol`, and `list_endpoints`
- verifies filters, truncation, and safe empty results at the engine boundary

## Run tests

```bash
uv run pytest
```

Run the focused TypeScript runtime tests:

```bash
npm run mcp-server:test
```

Run the cross-runtime parity scaffold:

```bash
npm run contract-tests
```

Run the Rust engine suite:

```bash
cargo test --manifest-path crates/navigation-engine/Cargo.toml
```

Run a single module:

```bash
uv run pytest tests/test_inspect_tree_contract.py
```

## Test design rules

- prefer temporary directories over repository-coupled fixtures
- prefer contract assertions over snapshot-style dumps
- avoid brittle integration tests against external analyzers unless the contract requires them
- keep tests small enough that future V2 refactors can preserve behavior without rewriting the suite

## What is intentionally not covered yet

- full end-to-end execution of every internal analyzer wrapper
- exhaustive backend error translation across every adapter path
- performance benchmarking

Those areas can be expanded later, but the current migration checkpoint should remain protected FIRST by contract-focused tests and the Rust engine suite.

## Current environment caveat

The inspect-tree parity check depends on the Python oracle being importable. In the current repository state, that means the Python `mcp` dependency must be installed. If it is missing, that parity test skips instead of failing with a misleading contract regression.

The migrated `code.find_symbol` and `code.list_endpoints` runtime paths do not depend on Python.
