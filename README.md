# @navigation-agent/mcp-server

Workspace-only MCP server for structural code navigation and repository inspection. It exposes a stable public `code.*` tool surface for finding symbol definitions, tracing upstream callers for impact analysis, tracing downstream execution flow before logic changes, listing routes/endpoints, searching text, and inspecting workspace trees without opening files blindly.

**npm:** [`@navigation-agent/mcp-server`](https://www.npmjs.com/package/@navigation-agent/mcp-server)

---

## Installation

The server runs via `npx`.

### Requirements

- **Node.js 18+**
- **[ripgrep](https://github.com/BurntSushi/ripgrep)** (`rg`) — optional, only needed for `code.search_text`

### Claude Code

```bash
claude mcp add --transport stdio navigation-agent -- npx -y @navigation-agent/mcp-server
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
      "enabled": true,
      "timeout": 30000
    }
  }
}
```

### Gemini CLI

```bash
gemini mcp add navigation-agent npx -- -y @navigation-agent/mcp-server
```

Or add it manually to `~/.gemini/settings.json` or `.gemini/settings.json`:

```json
{
  "mcpServers": {
    "navigation-agent": {
      "command": "npx",
      "args": ["-y", "@navigation-agent/mcp-server"],
      "timeout": 30000
    }
  }
}
```

Use the hyphenated server name `navigation-agent`. Avoid underscores in Gemini MCP server names because Gemini derives fully-qualified tool names from the server name.

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
startup_timeout_sec = 30
tool_timeout_sec = 60
```

### Workspace root

By default the server analyzes the current working directory. To pin a specific project, set `NAVIGATION_MCP_WORKSPACE_ROOT` in your MCP config.

---

## Agent usage guide

This server is built for **model-controlled MCP tool use**: agents should discover and invoke its tools before opening source files when the task is about workspace code structure.

It publishes both per-tool descriptions and server instructions so MCP clients can teach the model the workflow without relying on a private skill registry.

### How agents learn to use it

MCP clients give the model guidance through a few standard channels:

| Channel | What this server provides |
|---|---|
| MCP `initialize.result.instructions` | A concise workflow: use navigation before reading files, which tool to pick, supported languages/frameworks, and workspace-only limits. |
| Tool descriptions and input schemas | Each `code.*` tool explains when to use it and lists supported `language` / `framework` filters. |
| Structured tool results | Every tool returns the stable envelope `tool`, `status`, `summary`, `data`, `errors`, and `meta` so agents can chain outputs safely. |
| Optional client rules/skills | Clients such as OpenCode, Codex, Cursor, and Gemini can add project rules, but this server does not require a private registry to be useful. |

The important part: the server instructions are part of the MCP handshake, so clients that honor MCP instructions can inject them into the model before tool selection.

`skills/navigation-mcp/SKILL.md` is an optional portable skill template for clients that support skills. It is not required for normal MCP operation; for OpenCode specifically, skills are discovered from `.opencode/skills/<name>/SKILL.md`, global OpenCode skills, or Claude/agents-compatible skill directories.

### Quick path for agents

1. Use `code.inspect_tree` to orient in an unknown module or directory without reading files.
2. Use `code.find_symbol` when you know a class, function, method, type, enum, or annotation name but not the defining file.
3. Pass `find_symbol`'s returned `items[].path` into:
   - `code.trace_callers` for upstream impact: **who calls this?**
   - `code.trace_flow` for downstream behavior: **what does this call or reach?**
4. Use `code.list_endpoints` before changing REST, GraphQL, or route surfaces.
5. Use `code.search_text` for textual patterns, imports, decorators, or when symbol lookup is not enough.
6. Read only the relevant files returned by the navigation tools.

### Fallbacks agents should use

| Situation | Correct fallback |
|---|---|
| `code.find_symbol` returns zero for constants, config keys, decorators, imports, or generated names | Use `code.search_text` scoped by `path`, `include`, and `language`. |
| A trace result is too broad or noisy | Narrow `path`, `language`, `framework`, or `symbol`; for `trace_callers`, lower `max_depth`. |
| A route or endpoint inventory returns zero | Retry with a narrower `path` and the most specific `framework` or `kind` before concluding there is no public surface. |
| A navigation result has `truncated: true` | Narrow the query before reading files or increasing `limit`. |

Do not treat an empty result as proof by itself. Use one scoped fallback, then explain the limitation if results still stay empty.

### Tool naming in clients

The canonical public contract is `code.*`. Some clients expose MCP tools with a server prefix or normalized separators, for example `navigation-agent_code_find_symbol` or `mcp_navigation-agent_code.find_symbol`. Treat those names as aliases of the same canonical tools.

Use `navigation-agent` as the server name in examples. It is readable, avoids collisions, and avoids underscore-related parser issues in clients that derive fully-qualified tool names from the server id.

### Client convention notes

| Client | Convention checked |
|---|---|
| Claude Code | Local stdio command uses `claude mcp add --transport stdio <name> -- <command> <args...>`. Server instructions help Claude's MCP tool search decide when to load these tools. |
| OpenCode | Local MCP servers live under the `mcp` config key with `type: "local"` and `command` as an array. MCP tools are exposed with a server-name prefix, so prompts/rules can say “use `navigation-agent`”. |
| Gemini CLI | MCP servers live under `mcpServers`; stdio uses `command` + `args`. Gemini appends MCP server instructions to system instructions and assigns names like `mcp_{serverName}_{toolName}`. |
| Cursor | MCP servers are configured in `mcp.json` with `command` + `args` for stdio or `url` + `headers` for remote servers. |
| OpenAI Codex | MCP servers live under `[mcp_servers.<name>]` in `config.toml`; `codex mcp add <name> -- <command>` is the CLI form. |

### Supported filters agents should know

- Languages: `typescript`, `javascript`, `go`, `java`, `php`, `python`, `rust`, `csharp`
- Frameworks: `react-router`, `spring`

Do not use this MCP for web search, external repositories, arbitrary filesystem access, or reading file contents. It is a workspace-only navigation layer.

---

## Compatibility matrix

This table MUST stay in the README because it is the fastest way to understand the public support surface. It is intentionally organized with **languages as rows** and **tools as columns** so adding more languages grows downward instead of widening the table.

Tool columns omit the `code.` prefix to keep the matrix readable.

| Language | `inspect_tree` | `find_symbol` | `search_text` | `list_endpoints` | `trace_flow` | `trace_callers` |
|---|---:|---:|---:|---:|---:|---:|
| Java | ✅ | ✅ | ✅ | ✅ Spring REST/GraphQL | ✅ | ✅ |
| TypeScript | ✅ | ✅ | ✅ | ✅ React Router | ✅ | ✅ |
| JavaScript | ✅ | ✅ | ✅ | ✅ React Router | ✅ | ✅ |
| PHP | ✅ | ✅ | ✅ | ⚠️ Public, not re-verified for endpoints | ✅ | ✅ |
| Python | ✅ | ✅ | ✅ | ✅ FastAPI/Flask-style decorators | ✅ | ✅ |
| Rust | ✅ | ✅ | ✅ | ⚠️ Target-dependent / non-web target returned zero | ✅ Qualified symbols | ✅ Qualified symbols |
| Go | ✅ | ✅ | ✅ | ⚠️ No useful endpoint inventory in current example | ✅ | ✅ |
| C# | ✅ | ✅ | ✅ | ⚠️ Stub implementation | ✅ | ✅ |

Legend:

- ✅ = verified in a real project during this documentation sync
- ⚠️ = publicly exposed, but not re-verified in this pass, not meaningful on the chosen validation project, or still has caveats
- ❌ = not working as public support today
- `code.inspect_tree` and `code.search_text` also work without a language filter across general workspace files.

Important:

- Public language filters are `typescript`, `javascript`, `go`, `java`, `php`, `python`, `rust`, and `csharp`.
- Go, PHP, Python, Rust, Java, TypeScript, JavaScript, and C# are part of the public contract; the matrix above shows current verification level per tool.
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

### Before changing a function or method

Use this workflow whenever you need to understand behavior or impact inside the workspace:

1. `code.find_symbol` — resolve the exact defining file first.
2. `code.trace_callers` — inspect upstream impact before renaming, deleting, or changing a signature.
3. `code.trace_flow` — inspect downstream execution before changing logic.
4. `read` only the files returned by the trace results that actually matter.

Rule of thumb:

- choose `code.trace_callers` for **who depends on this?**
- choose `code.trace_flow` for **what does this reach or invoke?**
- if you need both impact and behavior, run both before editing

Concrete workspace example:

1. Resolve the symbol definition:

```json
{
  "symbol": "create_order",
  "language": "python",
  "kind": "function",
  "path": "examples/python"
}
```

2. Inspect upstream impact before changing the function:

```json
{
  "path": "examples/python/app/api/endpoints.py",
  "symbol": "create_order",
  "language": "python",
  "recursive": true,
  "max_depth": 3
}
```

3. Inspect downstream behavior before changing the logic:

```json
{
  "path": "examples/python/app/api/endpoints.py",
  "symbol": "create_order",
  "language": "python"
}
```

Expected agent behavior:

- use `code.find_symbol` first when the defining file is not already known
- use `code.trace_callers` first when the risk is breaking callers
- use `code.trace_flow` next when the risk is changing downstream behavior
- only then `read` the traced files you actually need

React Router example:

```json
{
  "symbol": "action",
  "kind": "function",
  "framework": "react-router",
  "path": "app/routes"
}
```

```json
{
  "path": "app/routes/change-password.tsx",
  "symbol": "action",
  "framework": "react-router",
  "recursive": true,
  "max_depth": 2
}
```

```json
{
  "path": "app/routes/change-password.tsx",
  "symbol": "action",
  "framework": "react-router"
}
```

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
- `code.list_endpoints` inventories framework-detectable Spring REST controllers and GraphQL resolvers as likely public entrypoints
- `code.trace_flow` works on real controller/resolver entrypoints
- `code.trace_callers` works on Java use cases and can identify probable public entrypoints

Verified example:
- `RootUserGraphQLController#getUsersByDependency`
- traced into `RootGetUserUseCase#getUsers`

### TypeScript / React Router (`~/sias/app/front`)

- `code.inspect_tree` works on real route trees
- `code.find_symbol` works on route-module exports
- `code.search_text` works on real route files
- `code.list_endpoints` inventories React Router route-module `loader` / `action` exports as likely route entrypoints
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
- `code.list_endpoints` inventories FastAPI-style route decorators (`@router.get`, `@app.post`, etc.) as likely public API entrypoints
- `code.trace_flow` works end-to-end for multi-module Python scenarios, capturing deep recursive trees including branching calls, instance methods (`self`), and cross-file resolution.
- `code.trace_callers` works end-to-end for impact analysis, supporting recursive reverse-tracing across the entire workspace.

Verified example (Forward Trace):
- `app/api/endpoints.py#create_order`
- traced into `order_service.process_order(...)` -> `_handle_payment(...)` -> `payment_service.authorize_payment(...)`
- deep tree captures cross-file calls to `AuditService`, `InventoryService`, `ProductRepository`, etc.

Verified example (Backward Trace):
- `app/services/audit.py#log_action`
- reverse-traced to callers in `UserService`, `OrderService`
- recursively identifies entrypoints in `app/api/endpoints.py` (`get_user`, `create_order`)

### PHP (`examples/php`)

- `code.inspect_tree` works on PHP project trees
- `code.find_symbol` works on PHP classes and methods
- `code.search_text` works on PHP source files
- `code.trace_flow` works end-to-end for PHP service-to-repository calls
- `code.trace_callers` works end-to-end for PHP impact analysis

Notes:
- `code.list_endpoints` is publicly exposed for PHP but was not re-verified for useful endpoint inventory in the current example

Verified example:
- `src/Service/UserService.php#UserService::persistUser`
- traced `$this->repository->save($user)` to `src/Repository/MemoryUserRepository.php#save`

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

### C# (`./examples/csharp`)

- `code.inspect_tree` works
- `code.search_text` works
- `code.find_symbol` works for method lookup such as `OrderWorkflowService.ProcessOrderAsync`
- `code.trace_flow` works end-to-end on the example app and returns the recursive internal call tree
- `code.trace_callers` works end-to-end on the example app

### Go (`./examples/go`)

Real behavior today against `examples/go`:

- `code.inspect_tree` works
- `code.search_text` works
- `code.find_symbol` works for method lookup such as `CreateUser`
- `code.trace_flow` works end-to-end on the example app and returns the recursive internal call tree
- `code.trace_callers` works end-to-end on the example app, including callback/method-value references and interface-to-implementation reverse matches
- `code.list_endpoints` still returns no useful entrypoint inventory for the current Go example

---

## Public language and framework filters

Current public language filters:

- `typescript`
- `javascript`
- `go`
- `java`
- `php`
- `python`
- `rust`
- `csharp`

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
