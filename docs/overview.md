# Navigation Agent MCP Overview

## What this MCP is

`navigation-agent-mcp` is a focused Model Context Protocol server for **code navigation and repository inspection**.

It exposes a stable `workspace.*` public contract designed for agents and automation that need to:

- locate symbols
- inspect repository trees safely
- index endpoints and routes
- search text with normalized results
- trace symbols forward
- trace callers backward

## Philosophy

This implementation is intentionally focused.

- **Navigation first**: inspect and trace code safely before adding heavier workflows.
- **Normalized envelopes**: every public tool returns the same top-level shape.
- **Machine-friendly semantics**: stable error codes, count metadata, and detection fields.
- **Workspace safety**: scoped paths must stay inside the configured workspace root.
- **No raw backend leakage**: internal analyzer details stay private to the implementation.

## Public tools

| Tool | Purpose |
| --- | --- |
| `workspace.find_symbol` | Locate symbol definitions by name |
| `workspace.search_text` | Search plain text or regex across files |
| `workspace.trace_flow` | Trace a symbol forward from a known file |
| `workspace.trace_callers` | Trace direct or recursive incoming callers |
| `workspace.list_endpoints` | Discover backend endpoints and frontend routes |
| `workspace.inspect_tree` | Inspect the workspace tree without reading file contents |

## Stable response contract

Every public tool returns:

```json
{
  "tool": "workspace.inspect_tree",
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

## Architecture

```text
client
  -> TypeScript runtime (`packages/mcp-server`)
    -> public request validation
    -> normalized response envelopes
    -> JSON over stdio
      -> Rust engine (`crates/navigation-engine`)
        -> workspace.inspect_tree
        -> workspace.find_symbol
        -> workspace.list_endpoints
        -> workspace.search_text
        -> workspace.trace_flow
        -> workspace.trace_callers
```

All six public tools run through the TS -> Rust path.

## Supported frameworks and languages

- **TypeScript/JavaScript**: React Router 7 routes
- **Java**: Spring REST and GraphQL endpoints
- **Python**: FastAPI, Flask, Django decorators and URL patterns
- **Rust**: Actix-web and async-graphql

## Intended audience

- agent developers integrating a stable code-navigation MCP
- maintainers evolving future versions safely from a hardened contract
- users who need inspection and trace workflows without exposing raw analyzer internals
