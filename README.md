# @navigation-agent/mcp-server

MCP server for code navigation and repository inspection. It exposes a stable public `code.*` tool surface for finding symbols, listing routes/endpoints, tracing execution flow, tracing callers, searching text, and inspecting workspace trees without opening files blindly.

**npm:** [`@navigation-agent/mcp-server`](https://www.npmjs.com/package/@navigation-agent/mcp-server)

---

## Installation

The server runs via `npx`.

### Requirements

- **Node.js 18+**
- **[ripgrep](https://github.com/BurntSushi/ripgrep)** (`rg`) — optional, only needed for `code.search_text`

### Claude Code

```bash
claude mcp add navigation-agent npx -- -y @navigation-agent/mcp-server
```

### OpenCode

Add to `~/.config/opencode/opencode.json`:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "navigation-agent": {
      "type": "local",
      "command": ["npx", "-y", "@navigation-agent/mcp-server"],
      "enabled": true
    }
  }
}
```

### Gemini CLI

Add to `~/.gemini/settings.json`:

```json
{
  "mcpServers": {
    "navigation-agent": {
      "command": "npx",
      "args": ["-y", "@navigation-agent/mcp-server"]
    }
  }
}
```

### Cursor

Add to `~/.cursor/mcp.json` or `.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "navigation-agent": {
      "command": "npx",
      "args": ["-y", "@navigation-agent/mcp-server"]
    }
  }
}
```

### OpenAI Codex

```bash
codex mcp add navigation-agent -- npx -y @navigation-agent/mcp-server
```

Or add to `~/.codex/config.toml`:

```toml
[mcp_servers.navigation-agent]
command = "npx"
args = ["-y", "@navigation-agent/mcp-server"]
```

### Workspace root

By default the server analyzes the current working directory. To pin a specific project, set `NAVIGATION_MCP_WORKSPACE_ROOT` in your MCP config.

---

## Compatibility matrix

This table MUST stay in the README because it is the fastest way to understand the public support surface.

| Capability | Java | TypeScript / JavaScript | Python | Rust | Go | All Files |
|---|---|---|---|---|---|---|
| `code.inspect_tree` | ✅ Verified on real Spring project tree | ✅ Verified on real React Router project tree | ✅ Verified on `examples/python` tree | ✅ Verified on this repository | ✅ Verified on `examples/go` tree | ✅ |
| `code.find_symbol` | ✅ Verified on real Spring code | ✅ Verified on real React Router code | ✅ Verified on `examples/python` symbols | ✅ Verified on this repository | ✅ Verified on `examples/go` method lookup | — |
| `code.search_text` | ✅ Verified on real Spring code | ✅ Verified on real React Router code | ✅ Verified on `examples/python` source | ✅ Verified on this repository | ✅ Verified on `examples/go` text search | ✅ |
| `code.list_endpoints` | ✅ Verified on real Spring REST / GraphQL code | ✅ Verified on real React Router route modules | ✅ Verified on `examples/python` FastAPI-style routes | ⚠️ Correctly returns no endpoints for this Rust engine project | ⚠️ Responds, but the current Go example has no useful endpoint detection yet | — |
| `code.trace_flow` | ✅ Verified on real Spring code | ✅ Verified on real React Router route flow | ✅ Verified end-to-end on `examples/python` with FastAPI routes and service calls | ✅ Verified on this repository with qualified Rust symbols | ✅ Verified end-to-end on `examples/go` | — |
| `code.trace_callers` | ✅ Verified on real Spring code | ✅ Verified on real React Router helper callers | ⚠️ Publicly exposed, but not yet re-validated for complex cross-file Python scenarios | ✅ Verified on this repository with qualified Rust symbols | ✅ Verified end-to-end on `examples/go` | — |

Legend:

- ✅ = verified in a real project during this documentation sync
- ⚠️ = publicly exposed, but not re-verified in this pass, not meaningful on the chosen validation project, or still has caveats
- ❌ = not working as public support today
- — = language-specific parsing not required

Important:

- Go is now part of the public contract and works well for symbol lookup, text search, trace flow, and trace callers on the validated example app.
- Rust trace tools work well, but method/impl symbols should be queried with their qualified name (for example `JavaProjectIndex::build`).

---

## Public tools

The public contract exposes exactly these six tools:

- `code.inspect_tree`
- `code.list_endpoints`
- `code.find_symbol`
- `code.search_text`
- `code.trace_flow`
- `code.trace_callers`

Use `snake_case` parameters such as `max_depth`, `include_hidden`, and `file_pattern`.

### `code.search_text` response style

`code.search_text` is optimized for agents:

- results are grouped by file
- each match returns only `line` plus exact `spans`
- `topFiles` highlights the densest files first
- contextual `before` / `after` text is intentionally omitted from the public response to reduce noise and token cost

Example shape:

```json
{
  "fileCount": 3,
  "matchCount": 19,
  "totalFileCount": 3,
  "totalMatchCount": 19,
  "topFiles": [
    {
      "path": "examples/go/internal/http/handlers/user_handler.go",
      "language": "go",
      "matchCount": 11
    }
  ],
  "items": [
    {
      "path": "examples/go/internal/http/handlers/user_handler.go",
      "language": "go",
      "matchCount": 11,
      "matches": [
        {
          "line": 28,
          "spans": [{ "colInit": 23, "colEnd": 32 }]
        }
      ]
    }
  ]
}
```

### Quick examples

```json
{
  "symbol": "RootUserGraphQLController",
  "language": "java",
  "kind": "class"
}
```

```json
{
  "path": "app/routes/change-password.tsx",
  "symbol": "action",
  "framework": "react-router"
}
```

```json
{
  "path": "src/main/java/com/example/FooController.java",
  "symbol": "getFoo",
  "framework": "spring"
}
```

---

## Verified real-world behavior

These checks were verified against real projects instead of toy stubs:

### Java (`~/sias/app/back`)

- `code.inspect_tree` works on real module trees
- `code.find_symbol` works on real Spring classes
- `code.search_text` works on real Java source
- `code.list_endpoints` works against Spring controllers and GraphQL resolvers
- `code.trace_flow` works on real controller/resolver entrypoints
- `code.trace_callers` works on Java use cases and can identify probable public entrypoints

Verified example:
- `RootUserGraphQLController#getUsersByDependency`
- traced into `RootGetUserUseCase#getUsers`

### TypeScript / React Router (`~/sias/app/front`)

- `code.inspect_tree` works on real route trees
- `code.find_symbol` works on route-module exports
- `code.search_text` works on real route files
- `code.list_endpoints` works for route-module `loader` / `action`
- `code.trace_flow` works for same-file route flow extraction
- `code.trace_callers` works for same-file helpers and marks route exports as probable entrypoints

Verified example:
- `app/routes/change-password.tsx#action`
- found calls to `getUserIdAndTokenFromSession`, `changeMyPassword`, `getSession`, `commitSession`, `getRoleRoute`
- reverse-traced `getRoleRoute <- action`

### Python (`examples/python`)

- `code.inspect_tree` works on Python module trees
- `code.find_symbol` works on Python classes, functions, and methods
- `code.search_text` works on Python source files
- `code.list_endpoints` works against FastAPI-style route decorators (`@router.get`, `@app.post`, etc.)
- `code.trace_flow` works end-to-end for cross-file Python scenarios, including:
  - Function calls and method invocations with receiver detection
  - Async functions and async calls
  - Decorated functions (FastAPI routes, dataclasses, etc.)
  - Multi-level call chains across modules
- `code.trace_callers` is publicly exposed but not yet re-validated for complex cross-file Python scenarios

Verified example:
- `app/api/endpoints.py#get_product`
- traced into `inventory_service.get_product_by_id(product_id)`
- resolves to `app/services/inventory.py#InventoryService.get_product_by_id`
- captures receiver type (`inventory_service`) and method name (`get_product_by_id`)

### Rust (this repository)

- `code.inspect_tree` works on real Rust source trees
- `code.find_symbol` works on Rust types/functions
- `code.search_text` works on real Rust source
- `code.trace_flow` works on real Rust methods when queried with the correct qualified symbol
- `code.trace_callers` works on real Rust methods when queried with the correct qualified symbol

Notes:
- `code.list_endpoints` returned zero results on this repository, which is expected for the chosen validation target because it is not a Rust web app

Verified example:
- `crates/navigation-engine/src/capabilities/trace_flow.rs#JavaProjectIndex::build`
- traced `Self::new_empty()`, `index.scan_project(workspace_root)`, and `index.is_empty()`
- reverse-traced `JavaProjectIndex::scan_project <- JavaProjectIndex::build`

### Go (`./examples/go`)

Real behavior today against `examples/go`:

- `code.inspect_tree` works
- `code.search_text` works
- `code.find_symbol` works for method lookup such as `CreateUser`
- `code.trace_flow` works end-to-end on the example app and returns the recursive internal call tree
- `code.trace_callers` works end-to-end on the example app, including callback/method-value references and interface-to-implementation reverse matches
- `code.list_endpoints` still returns no useful endpoint detection for the current Go example

---

## Public language and framework filters

Current public language filters:

- `typescript`
- `javascript`
- `go`
- `java`
- `python`
- `rust`

Current public framework filters:

- `react-router`
- `spring`

---

## Response shape

Every tool returns the same top-level envelope:

```json
{
  "tool": "code.trace_flow",
  "status": "ok",
  "summary": "Traced 5 callees for 'action' from 'app/routes/change-password.tsx'.",
  "data": {},
  "errors": [],
  "meta": {
    "query": {},
    "resolvedPath": "app/routes/change-password.tsx",
    "truncated": false,
    "counts": {},
    "detection": {}
  }
}
```

Status meanings:

- `ok` — request succeeded, including zero-result success
- `partial` — request succeeded but was truncated/pruned
- `error` — request failed and includes stable error codes

Notes:

- `code.trace_flow` returns a rooted recursive tree under `data.root`
- `code.trace_callers` returns direct callers plus recursive reverse-trace metadata
- `code.search_text` returns compact grouped matches and `topFiles`, not full context blocks

---

## Architecture

This repository has two main layers:

1. **TypeScript MCP runtime** (`packages/mcp-server/`)
   - validates the public `code.*` contract
   - exposes stdio / stdio-legacy transports
   - normalizes responses

2. **Rust engine** (`crates/navigation-engine/`)
   - parses source with tree-sitter
   - hosts language analyzers
   - includes internal AST/debug binaries under `crates/navigation-engine/src/bin/`

Important:

- `packages/mcp-server/src/bin/` contains the runtime entrypoint (`navigation-mcp.ts`)
- AST inspection/debug binaries live in `crates/navigation-engine/src/bin/`, not in the TypeScript runtime

---

## Contributing / local development

Key local commands:

```bash
npm install
npm --workspace @navigation-agent/mcp-server run check
npm --workspace @navigation-agent/mcp-server run test
cargo test --manifest-path crates/navigation-engine/Cargo.toml
```

Useful local runtime checks:

```bash
npx tsx packages/mcp-server/src/bin/navigation-mcp.ts --describe-tools
npx tsx packages/mcp-server/src/bin/navigation-mcp.ts --transport stdio-legacy --workspace-root /path/to/workspace
```

## License

MIT
