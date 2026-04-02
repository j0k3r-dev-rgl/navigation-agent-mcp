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

Important:

- Go analyzer work exists internally in Rust, but **Go is not part of the public TS contract yet**.
- Rust `code.trace_callers` is exposed publicly, but the real validated case still showed incomplete behavior.

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

### Rust (this repository)

- `code.inspect_tree` works on real Rust source trees
- `code.find_symbol` works on Rust types/functions
- `code.search_text` works on real Rust source
- `code.trace_flow` works on real Rust methods, including the recent local-binding slice in `trace_flow.rs`
- `code.trace_callers` is exposed publicly, but the real Rust case validated here still returned no callers for symbols that do have callers in code

Notes:
- `code.list_endpoints` returned zero results on this repository, which is expected for the chosen validation target because it is not a Rust web app

Verified example:
- `crates/navigation-engine/src/capabilities/trace_flow.rs#build`
- traced `Self::new_empty()`, `index.scan_project(workspace_root)`, and `index.is_empty()`

### Go (`./examples/go`)

Go analyzer work exists in the Rust engine, but **Go is not part of the public TypeScript contract yet**.

Real behavior today against `examples/go`:

- `code.inspect_tree` works
- `code.search_text` works
- `code.find_symbol` did **not** find `UserHandler`
- `code.find_symbol` also did **not** find `CreateUser`
- `code.list_endpoints` returned no useful support
- `code.trace_flow` on `CreateUser` returned `ok` but with **no callees**
- `code.trace_callers` on `NewUser` returned **backend execution failed**

So Go should be documented as **work in progress / not publicly supported yet**, not as a stable supported language.

---

## Public language and framework filters

Current public language filters:

- `typescript`
- `javascript`
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

See:

- [docs/development.md](docs/development.md)
- [docs/overview.md](docs/overview.md)

## License

MIT
