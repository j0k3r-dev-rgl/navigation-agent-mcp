# @navigation-agent/mcp-server

Workspace-only MCP server for structural code navigation. It helps CLI agents inspect a repository before opening files by exposing a stable `code.*` tool surface for tree inspection, symbol lookup, endpoint discovery, text search, caller impact, and downstream flow tracing.

Repository: https://github.com/j0k3r-dev-rgl/navigation-agent-mcp

## Quick Start

Run it through `npx` from any MCP client:

```bash
npx -y @navigation-agent/mcp-server
```

The server uses stdio by default. By default it analyzes the MCP process current working directory. To pin a project, set `NAVIGATION_MCP_WORKSPACE_ROOT` in your MCP client config.

Requirements:

- Node.js 18+
- `rg` from ripgrep, optional and only needed for `code.search_text`

## CLI Agent Setup

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

## Agent Usage

This package does not require a private skill registry. It publishes MCP server instructions and per-tool descriptions so clients that honor MCP instructions can teach the model when and how to use the tools.

Optional skill users can copy the portable skill from the repository: `skills/navigation-mcp/SKILL.md`.

Use this workflow:

1. Use `code.inspect_tree` to orient in an unknown module without reading files.
2. Use `code.find_symbol` when you know a class, function, method, type, enum, or annotation name.
3. Pass `find_symbol` results into `code.trace_callers` for upstream impact or `code.trace_flow` for downstream behavior.
4. Use `code.list_endpoints` before changing REST, GraphQL, or route surfaces.
5. Use `code.search_text` for textual patterns, imports, decorators, or when symbol lookup is not enough.
6. Read only the relevant files returned by the navigation tools.

Fallbacks agents should use:

| Situation | Correct fallback |
|---|---|
| `code.find_symbol` returns zero for constants, config keys, decorators, imports, or generated names | Use `code.search_text` scoped by `path`, `include`, and `language`. |
| A trace result is too broad or noisy | Narrow `path`, `language`, `framework`, or `symbol`; for `trace_callers`, lower `max_depth`. |
| A route or endpoint inventory returns zero | Retry with a narrower `path` and the most specific `framework` or `kind` before concluding there is no public surface. |
| A navigation result has `truncated: true` | Narrow the query before reading files or increasing `limit`. |

Do not treat an empty result as proof by itself. Use one scoped fallback, then explain the limitation if results still stay empty.

Canonical tools:

- `code.inspect_tree`
- `code.find_symbol`
- `code.list_endpoints`
- `code.search_text`
- `code.trace_flow`
- `code.trace_callers`

Some clients expose MCP tools with a server prefix or normalized separators, such as `navigation-agent_code_find_symbol`. Treat those names as aliases of the canonical `code.*` tools.

Supported filters:

- Languages: `typescript`, `javascript`, `go`, `java`, `php`, `python`, `rust`, `csharp`
- Frameworks: `react-router`, `spring`

Do not use this MCP for web search, external repositories, arbitrary filesystem access, or reading file contents. It is a workspace-only navigation layer.

## Inspect Published Tool Guidance

```bash
npx -y @navigation-agent/mcp-server --describe-tools
```

This prints the MCP instructions, tool names, input schemas, and descriptions that CLI agents receive from the server.
