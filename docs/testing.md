# Testing Guide

## Goal

The test suite protects the **public V1 contract**, not internal implementation trivia.

That means the highest-value tests focus on:

- MCP tool discoverability and schema shape
- normalized envelopes and stable metadata
- `find_symbol` exact/fuzzy/language-framework semantics
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
├── findSymbolContract.spec.ts
├── inspectTreeContract.spec.ts
├── findSymbolService.spec.ts
└── toolRegistration.spec.ts

packages/contract-tests/test/
└── public-contract.spec.ts
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

### `packages/mcp-server/test/findSymbolService.spec.ts`

- verifies migrated request shaping for `react-router`, `javascript`, and `spring`
- verifies stable summary, truncation, and error-envelope mapping at the TS service layer

### `packages/mcp-server/test/findSymbolContract.spec.ts`

- verifies the migrated `code.find_symbol` stdio runtime path without Python
- verifies Java framework inference and partial-result envelopes at the public runtime boundary

### `packages/mcp-server/test/toolRegistration.spec.ts`

- verifies the npm-first TypeScript runtime exposes the same six `code.*` tools
- verifies the discoverable schema defaults/required fields remain stable

### `packages/mcp-server/test/inspectTreeContract.spec.ts`

- verifies the migrated `code.inspect_tree` path through the TS runtime
- verifies hidden-vs-hard-ignored behavior, truncation, and missing-path errors

### `packages/contract-tests/test/public-contract.spec.ts`

- compares the TypeScript `code.inspect_tree` output with the Python oracle when that oracle is available
- verifies the Sprint 2 `code.find_symbol` public contract through the TypeScript runtime without requiring Python
- skips with the exact blocking reason when Python MCP dependencies are missing in the environment

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

Those areas can be expanded later, but the current migration checkpoint should remain protected FIRST by contract-focused tests.

## Current environment caveat

The inspect-tree parity check depends on the Python oracle being importable. In the current repository state, that means the Python `mcp` dependency must be installed. If it is missing, that parity test skips instead of failing with a misleading contract regression.

The migrated `code.find_symbol` contract tests do not depend on Python.
