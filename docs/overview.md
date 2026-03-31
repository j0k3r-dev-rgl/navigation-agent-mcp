# Navigation Agent MCP Overview

## What this MCP is

`navigation-agent-mcp` is a focused Model Context Protocol server for **code navigation and repository inspection**.

It exposes a stable `code.*` public contract designed for agents and automation that need to:

- locate symbols
- inspect repository trees safely
- index endpoints and routes
- search text with normalized results
- trace symbols forward
- trace callers backward

## Philosophy

V1 is intentionally narrow.

- **Navigation first**: inspect and trace code safely before adding heavier workflows.
- **Normalized envelopes**: every public tool returns the same top-level shape.
- **Machine-friendly semantics**: stable error codes, count metadata, and detection fields.
- **Workspace safety**: scoped paths must stay inside the configured workspace root.
- **No raw backend leakage**: internal analyzer details stay private to the implementation.

## Public V1 tools

| Tool | Purpose |
| --- | --- |
| `code.find_symbol` | Locate symbol definitions by name |
| `code.search_text` | Search plain text or regex across files |
| `code.trace_symbol` | Trace a symbol forward from a known file |
| `code.trace_callers` | Trace direct or recursive incoming callers |
| `code.list_endpoints` | Discover backend endpoints and frontend routes |
| `code.inspect_tree` | Inspect the workspace tree without reading file contents |

## Stable response contract

Every public tool returns:

```json
{
  "tool": "code.inspect_tree",
  "status": "ok",
  "summary": "Inspected 3 tree entries under '.'.",
  "data": {},
  "errors": [],
  "meta": {
    "query": {},
    "resolvedPath": ".",
    "truncated": false,
    "counts": {},
    "detection": {}
  }
}
```

### Status meanings

- `ok`: success, including zero-result success
- `partial`: success with truncation or safety pruning
- `error`: request could not be completed

### Common path guarantees

- Missing scoped paths return `FILE_NOT_FOUND`
- Escaping the workspace root returns `PATH_OUTSIDE_WORKSPACE`

## Architecture at a glance

```text
client
  -> TypeScript runtime (`packages/mcp-server`)
    -> public request validation
    -> normalized response envelopes
    -> JSON over stdio (current checkpoint transport)
      -> Rust engine (`crates/navigation-engine`)
        -> workspace.inspect_tree
        -> workspace.find_symbol
        -> workspace.list_endpoints
        -> workspace.search_text
        -> workspace.trace_symbol
        -> workspace.trace_callers
```

## Current migration status

| Tool | Status | Runtime |
| --- | --- | --- |
| `code.inspect_tree` | migrated | TS -> Rust |
| `code.find_symbol` | migrated | TS -> Rust |
| `code.list_endpoints` | migrated | TS -> Rust |
| `code.search_text` | migrated | TS -> Rust |
| `code.trace_symbol` | migrated | TS -> Rust |
| `code.trace_callers` | migrated | TS -> Rust |

`code.list_endpoints` now covers these migrated analyzer families:

- React Router 7 route discovery for TypeScript/JavaScript
- Spring REST and GraphQL discovery for Java
- Python decorator / URL-pattern discovery for FastAPI, Flask, and Django-style patterns
- Rust REST / GraphQL discovery for Actix-style attrs and async-graphql attrs

`code.trace_callers` now runs in Rust as well. Its current implementation uses AST/tree-sitter analysis for TypeScript and Java while preserving the public response contract; `implementationInterfaceChain` remains empty by design in the current engine behavior.

## Intended audience

- agent developers integrating a stable code-navigation MCP
- maintainers evolving V2+ safely from a hardened V1 contract
- users who need inspection and trace workflows without exposing raw analyzer internals
