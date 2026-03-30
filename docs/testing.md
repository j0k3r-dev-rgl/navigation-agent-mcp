# Testing Guide

## Goal

The test suite protects the **public V1 contract**, not internal implementation trivia.

That means the highest-value tests focus on:

- MCP tool discoverability and schema shape
- normalized envelopes and stable metadata
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

## Run tests

```bash
uv run pytest
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

Those areas can be expanded later, but V1 should remain protected FIRST by contract-focused tests.
