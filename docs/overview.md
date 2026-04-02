# Navigation Agent MCP Overview

## What this MCP is

`navigation-agent-mcp` is a focused MCP server for code navigation and repository inspection.

Its stable public contract is the `code.*` tool family:

- `code.find_symbol`
- `code.search_text`
- `code.trace_flow`
- `code.trace_callers`
- `code.list_endpoints`
- `code.inspect_tree`

## Philosophy

- **Navigation first**: inspect and trace before broad file reads
- **Normalized envelopes**: every tool returns the same top-level response shape
- **Machine-friendly semantics**: stable error codes, counts, and detection metadata
- **Workspace safety**: scoped paths must stay inside the configured workspace root
- **No raw backend leakage**: engine internals stay behind the public contract

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

### Common guarantees

- missing scoped paths return `FILE_NOT_FOUND`
- paths outside the workspace root return `PATH_OUTSIDE_WORKSPACE`
- request metadata is echoed back under `meta.query`

## Architecture

```text
client
  -> TypeScript runtime (`packages/mcp-server`)
    -> public request validation
    -> normalized response envelopes
    -> stdio / stdio-legacy transport
      -> Rust engine (`crates/navigation-engine`)
        -> analyzers per language
        -> endpoint discovery
        -> symbol lookup
        -> text search
        -> forward trace
        -> reverse caller trace
```

Important separation:

- `packages/mcp-server/src/bin/navigation-mcp.ts` is the runtime entrypoint
- analyzer debug/AST binaries live in `crates/navigation-engine/src/bin/`

## Public filters today

### Languages

- `typescript`
- `javascript`
- `java`
- `python`
- `rust`

### Frameworks

- `react-router`
- `spring`

## Compatibility matrix

| Capability | Java | TypeScript / JavaScript | Python | Rust | Go | All Files |
|---|---|---|---|---|---|---|
| `code.inspect_tree` | ✅ Verified on real Spring project tree | ✅ Verified on real React Router project tree | ⚠️ Publicly exposed, not re-validated in this pass | ✅ Verified on this repository | ✅ Verified on `examples/go` tree | ✅ |
| `code.find_symbol` | ✅ Verified on real Spring code | ✅ Verified on real React Router code | ⚠️ Publicly exposed, not re-validated in this pass | ✅ Verified on this repository | ❌ Real validation returned no symbol definitions | — |
| `code.search_text` | ✅ Verified on real Spring code | ✅ Verified on real React Router code | ⚠️ Publicly exposed, not re-validated in this pass | ✅ Verified on this repository | ✅ Verified on `examples/go` text search | ✅ |
| `code.list_endpoints` | ✅ Verified on real Spring REST / GraphQL code | ✅ Verified on real React Router route modules | ⚠️ Publicly exposed, not re-validated in this pass | ⚠️ Publicly exposed; chosen Rust project had no web endpoints to validate against | ❌ Real validation returned no useful endpoint support | — |
| `code.trace_flow` | ✅ Verified on real Spring code | ✅ Verified for same-file React Router route flow | ⚠️ Publicly exposed, not re-validated in this pass | ✅ Verified on this repository | ❌ Returns empty results in real `examples/go` validation | — |
| `code.trace_callers` | ✅ Verified on real Spring code | ✅ Verified for same-file helper callers | ⚠️ Publicly exposed, not re-validated in this pass | ⚠️ Exposed, but real validated case is still incomplete | ❌ Real validation fails | — |

Legend:

- ✅ = verified in a real project during this documentation sync
- ⚠️ = publicly exposed, but not re-verified in this pass, not meaningful on the chosen validation project, or still has caveats
- ❌ = not working as public support today
- — = language-specific parsing not required

## Real support snapshot

This is the important part: SUPPORT IS NOT JUST “code exists somewhere”. It must work through the public MCP surface.

### Verified in real projects

- **Java / Spring**
  - `list_endpoints` works on real controllers/resolvers
  - `trace_flow` works on real entrypoints
  - `trace_callers` works on real use-case methods

- **TypeScript / React Router**
  - `list_endpoints` works on route modules
  - `trace_flow` works for same-file route flow extraction
  - `trace_callers` works for same-file helpers and route exports

- **Rust**
  - `find_symbol` works in real code
  - `trace_flow` works in real code
  - `trace_callers` is exposed but the validated real case still returned no callers where callers exist, so treat it as incomplete

### Not publicly supported yet

- **Go**
  - there is analyzer work in the Rust engine and an `examples/go` app
  - but Go is not part of the public TS contract today
  - real runs currently return empty/missing results, so it should be treated as work in progress

## Intended audience

- agent developers integrating a stable code-navigation MCP
- maintainers evolving the public contract safely
- teams that need structural code discovery without handing raw repository reads to the model
