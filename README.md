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

## Supported Languages & Capabilities

| Capability | Java | TypeScript | Python | Rust | All Files |
|---|---|---|---|---|---|
| **workspace.find_symbol** | ✓ | ✓ | ✓ | ✓ | — |
| **workspace.list_endpoints** | ✓ Spring REST & GraphQL | ✓ React Router 7 | ✓ FastAPI, Flask | ✓ Actix | — |
| **workspace.trace_callers** | ✓ | ✓ | — | — | — |
| **workspace.trace_flow** | ✓ Full DI tracing | — | — | — | — |
| **workspace.inspect_tree** | — | — | — | — | ✓ |
| **workspace.search_text** | — | — | — | — | ✓ |

**Legend:**
- ✓ = Fully supported
- — = Not supported (returns empty results)
- **workspace.inspect_tree** and **workspace.search_text** work across all file types (no language parsing needed)

### Java Special Features
- **Full Spring DI tracing**: `trace_flow` follows interfaces to implementations through the complete call chain
- **Infrastructure boundary detection**: Automatically stops recursion at adapters/repositories
- **Method grouping**: Groups multiple calls to the same method with count

---

## Tools

### `workspace.find_symbol`
Find where a symbol (class, function, interface, etc.) is defined in the workspace.

**Example:** Find all implementations of `findUsersByRoot`
```json
{
  "symbol": "findUsersByRoot",
  "analyzerLanguage": "java"
}
```

**Returns:**
- Symbol name and kind (method, class, interface)
- File path
- Line start and line end (function boundaries)
- Language

---

### `workspace.inspect_tree`
Inspect the directory tree without reading file contents. Useful for orientation before diving in.

**Parameters:**
- `path`: Directory to inspect (optional, defaults to workspace root)
- `maxDepth`: Maximum depth to traverse (optional)
- `includeStats`: Include file size and modified time (optional)

---

### `workspace.list_endpoints`
List backend REST/GraphQL endpoints and frontend routes discovered by static analysis.

**Supported frameworks:**
- **Java**: Spring REST, Spring GraphQL
- **TypeScript**: React Router 7
- **Python**: FastAPI, Flask, Django
- **Rust**: Actix-web, async-graphql

---

### `workspace.search_text`
Search plain text or regex patterns across workspace files using ripgrep.

**Parameters:**
- `query`: Search pattern (string or regex)
- `path`: Scope path (optional)
- `caseSensitive`: Case-sensitive search (optional)
- `includePattern`: File pattern to include (optional, e.g., "*.java")

---

### `workspace.trace_flow` (Java only)
Trace a method's complete call flow through the application, following Spring DI interfaces to their implementations.

**Example:** Trace `getUsersByDependency` endpoint
```json
{
  "path": "src/main/java/.../RootUserGraphQLController.java",
  "symbol": "getUsersByDependency",
  "analyzerLanguage": "java"
}
```

**Features:**
- Follows interfaces (ports) to implementations (adapters)
- Stops recursion at infrastructure layer (adapters/repositories)
- Groups multiple calls to the same method
- Detects recursive calls
- Returns call depth for each callee

**Flow example:**
```
Controller.getUsersByDependency (depth 1)
  → Port.getUsers (depth 2)
    → UseCase.findUsersByRoot (depth 2)
      → Repository.findByIds (depth 2)
        → Adapter.findByIds (depth 3) [stops - infrastructure]
```

---

### `workspace.trace_callers`
Trace incoming callers for a symbol. Supports recursive traversal with configurable depth.

**Parameters:**
- `path`: Starting file path
- `symbol`: Symbol name to trace
- `analyzerLanguage`: Language ("java", "typescript", "python", "rust")
- `maxDepth`: Maximum recursion depth (optional, default 5)

---

## Architecture

The server uses a two-layer architecture:

1. **Rust Engine** (`crates/navigation-engine/`)
   - Fast parsing with tree-sitter
   - Language-specific analyzers (Java, TypeScript, Python, Rust)
   - Global project index for Java (interfaces, implementations)
   - JSON-RPC interface

2. **TypeScript Server** (MCP wrapper)
   - MCP protocol handling
   - Request/response marshalling
   - Tool registration

### Java Analysis Features
- **Interface indexing**: Scans all interfaces and their implementations
- **Field type resolution**: Resolves field types including wildcard imports
- **Builder chain detection**: Filters Lombok builder noise
- **Framework filtering**: Excludes `java.*`, `spring.*`, etc.

---

## License

MIT

---

> For contributing or running the server locally, see [docs/development.md](docs/development.md).
