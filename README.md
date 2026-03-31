# @navigation-agent/mcp-server

MCP server for code navigation and repository inspection. Gives any AI coding agent the ability to find symbols, trace call chains, list endpoints, search text, and inspect the workspace tree — without reading raw file contents.

**npm:** [`@navigation-agent/mcp-server`](https://www.npmjs.com/package/@navigation-agent/mcp-server)

---

## Installation

The server runs via `npx` — no global install needed. You only need **Node.js 24+** and **[ripgrep](https://github.com/BurntSushi/ripgrep)** (`rg`) on your system.

### Install ripgrep

| OS | Command |
|---|---|
| macOS | `brew install ripgrep` |
| Arch Linux | `sudo pacman -S ripgrep` |
| Ubuntu / Debian | `sudo apt install ripgrep` |
| Windows | `winget install BurntSushi.ripgrep.MSVC` |

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

Add to `~/.cursor/mcp.json` (global) or `.cursor/mcp.json` (per project):

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

By default the server analyzes the directory where your agent is running. To pin a specific path, pass `NAVIGATION_MCP_WORKSPACE_ROOT` as an environment variable in your agent's MCP config.

---

## Supported languages

| Language | find_symbol | list_endpoints | trace_callers |
|---|---|---|---|
| TypeScript / JavaScript | ✓ | ✓ React Router 7 | ✓ |
| Java | ✓ | ✓ Spring REST & GraphQL | ✓ |
| Python | ✓ | ✓ FastAPI, Flask, Django | — |
| Rust | ✓ | ✓ Actix, async-graphql | — |

`code.inspect_tree`, `code.search_text`, and `code.trace_symbol` work across all file types.

---

## Tools

### `code.find_symbol`
Find where a symbol (class, function, interface, etc.) is defined in the workspace.

### `code.inspect_tree`
Inspect the directory tree without reading file contents. Useful for orientation before diving in.

### `code.list_endpoints`
List backend REST/GraphQL endpoints and frontend routes discovered by static analysis.

### `code.search_text`
Search plain text or regex patterns across workspace files. Requires `ripgrep`.

### `code.trace_symbol`
Trace a symbol forward from a known file to see where it flows.

### `code.trace_callers`
Trace incoming callers for a symbol. Supports recursive traversal with configurable depth.

---

## License

MIT

---

> For contributing or running the server locally, see [docs/development.md](docs/development.md).
