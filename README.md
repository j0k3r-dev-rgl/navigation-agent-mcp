# navigation-agent-mcp

Minimal MCP server scaffold focused on code navigation and repository inspection.

## Purpose

This project exposes a **public, normalized MCP API** for analysis tools under the `code.*` namespace.

V1 scope:

- Navigation / analysis / inspection only
- First public tool: `code.find_symbol`
- Tree inspection tool: `code.inspect_tree`
- Endpoint index tool: `code.list_endpoints`
- Forward trace tool: `code.trace_symbol`
- Reverse trace tool: `code.trace_callers`
- Text search tool: `code.search_text`
- Structured responses for agent-friendly automation

## Tech Stack

- TypeScript runtime layer (npm-first)
- Rust engine boundary for migrated capabilities
- Python runtime kept as the compatibility path for non-migrated tools
- `uv` project layout for the legacy/oracle path
- Pydantic models for the existing public contract source of truth

## Project Layout

```text
packages/
├── mcp-server/                # npm-first TypeScript runtime shell
└── contract-tests/            # cross-runtime parity guards
crates/
└── navigation-engine/         # Rust capability engine
src/navigation_mcp/
├── adapters/internal_tools/   # legacy/internal wrappers kept as the oracle path
├── contracts/                 # public request/response models
├── services/                  # Python orchestration/oracle behavior
├── tools/                     # Python MCP tool registration
├── app.py                     # Python FastMCP assembly
└── server.py                  # Python CLI entrypoint
```

## Documentation

- `docs/overview.md` — product scope, philosophy, and public tools
- `docs/v1-summary.md` — shipped V1 surface, limitations, and tradeoffs
- `docs/release-checklist.md` — future release checklist
- `docs/testing.md` — test layout and commands
- `docs/migration/sprint-1.md` — current TS/Rust checkpoint, runtime boundary, and remaining follow-up

## Run

## Preferred runtime: npm-first TypeScript shell

Describe the public tool surface:

```bash
node --experimental-strip-types packages/mcp-server/src/bin/navigation-mcp.ts --describe-tools
```

Start the TypeScript runtime over stdio:

```bash
node --experimental-strip-types packages/mcp-server/src/bin/navigation-mcp.ts --transport stdio --workspace-root /path/to/workspace
```

Or through the npm workspace shortcut:

```bash
npm run mcp-server:dev -- --workspace-root /path/to/workspace
```

### Current migration boundary

- fully migrated through TS -> Rust: `code.inspect_tree`, `code.find_symbol`
- still on the Python compatibility path: `code.search_text`, `code.trace_symbol`, `code.trace_callers`, `code.list_endpoints`

### Rust engine command

By default the TypeScript runtime starts the Rust engine with:

```bash
cargo run --quiet --manifest-path crates/navigation-engine/Cargo.toml
```

Override it with `NAVIGATION_MCP_RUST_ENGINE_CMD` as a JSON string array when needed.

Example:

```bash
export NAVIGATION_MCP_RUST_ENGINE_CMD='["cargo","run","--quiet","--manifest-path","crates/navigation-engine/Cargo.toml"]'
```

## Legacy / oracle runtime: Python

### Stdio

```bash
uv run navigation-mcp --transport stdio
```

### Streamable HTTP

```bash
uv run navigation-mcp --transport streamable-http --host 127.0.0.1 --port 8000 --path /mcp
```

### Optional environment variables

- `NAVIGATION_MCP_WORKSPACE_ROOT`: workspace root to analyze. Defaults to the current working directory.
- `NAVIGATION_MCP_RUST_ENGINE_CMD`: JSON string array that overrides the Rust engine command for the TS runtime.
- `NAVIGATION_MCP_PYTHON`: optional Python executable override for the TS compatibility bridge.
- `NAVIGATION_MCP_FIND_SYMBOL_SCRIPT`: override the internal adapter script path.
- `NAVIGATION_MCP_LIST_ENDPOINTS_SCRIPT`: override the internal list_endpoints adapter script path.
- `NAVIGATION_MCP_TRACE_CALLERS_SCRIPT`: override the internal trace_callers adapter script path.
- `NAVIGATION_MCP_TRACE_SYMBOL_SCRIPT`: override the internal trace_symbol adapter script path.

## Test in local environments

If you want to try this MCP on another PC with the same OpenCode setup, the simplest path is to install it as a user command and then register it in OpenCode.

### Required programs

You need these programs installed on the machine:

- `node` 24+
- `npm`
- `python` 3.12+ (for the legacy/oracle path and compatibility bridge)
- `uv` (for the legacy/oracle path)
- `ripgrep`
- `opencode` (if you want to use it from OpenCode)

### Install on Arch Linux

```bash
sudo pacman -S python uv ripgrep opencode
```

If `opencode` is not available in your environment yet, install it using the official OpenCode method described in their docs.

### Install this MCP locally

For the current Sprint 2 checkpoint, the npm-first path is:

```bash
npm install
```

Then run:

```bash
npm run mcp-server:dev -- --workspace-root /path/to/workspace
```

The legacy Python install path is still available while the migration is in progress:

From the root of this repository:

```bash
uv tool install .
```

If it was already installed and you want to refresh it after changes:

```bash
uv tool install --reinstall .
```

This installs the `navigation-mcp` command in your user environment.

### Quick local verification

Check that the command is available:

```bash
navigation-mcp --help
```

For the npm-first runtime, verify the TypeScript shell exposes the stable tool surface:

```bash
node --experimental-strip-types packages/mcp-server/src/bin/navigation-mcp.ts --describe-tools
```

You can also start it manually over stdio:

```bash
navigation-mcp --transport stdio
```

### Enable it in OpenCode

Add this to `~/.config/opencode/opencode.json` on the target machine:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "navigation": {
      "type": "local",
      "command": ["navigation-mcp", "--transport", "stdio"],
      "enabled": true
    }
  }
}
```

If you want to pin the analyzed workspace explicitly, add an environment variable:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "navigation": {
      "type": "local",
      "command": ["navigation-mcp", "--transport", "stdio"],
      "enabled": true,
      "environment": {
        "NAVIGATION_MCP_WORKSPACE_ROOT": "/path/to/workspace"
      }
    }
  }
}
```

### Verify OpenCode detects it

```bash
opencode mcp list
```

Then open OpenCode in your project and ask it to use the navigation MCP for code analysis.

### Repeat on another PC

If the other machine uses the same OpenCode configuration style, the migration steps are:

1. Install `python`, `uv`, `ripgrep`, and `opencode`
2. Clone this repository
3. Run `uv tool install .`
4. Copy or recreate the `opencode.json` MCP entry
5. Copy your project skill/rules if you want the same behavior defaults

## Public Tool Contract

All tools return the same envelope shape:

```json
{
  "tool": "code.find_symbol",
  "status": "ok",
  "summary": "Found 2 symbol definitions for 'loader'.",
  "data": {},
  "errors": [],
  "meta": {
    "query": {},
    "resolvedPath": null,
    "truncated": false,
    "counts": {},
    "detection": {}
  }
}
```

### Stable `meta` contract

- `query`: normalized request payload
- `resolvedPath`: workspace-relative resolved scope when a `path` was provided
- `truncated`: `true` when data was truncated or safety-pruned
- `counts`: stable machine-readable count metadata
- `detection`: normalized derived metadata such as effective language/framework when meaningful

### Stable path semantics

If a scoped `path` is provided and it does not exist inside the configured workspace, the tool returns `status: "error"` with `FILE_NOT_FOUND`.

If the provided `path` resolves outside the workspace, the tool returns `status: "error"` with `PATH_OUTSIDE_WORKSPACE`.

### `code.find_symbol`

Find symbol definitions in the workspace.

#### Input

| Field | Type | Required | Notes |
| --- | --- | --- | --- |
| `symbol` | string | yes | Symbol name to locate |
| `language` | `typescript \| javascript \| java` | no | Optional language filter |
| `framework` | `react-router \| spring` | no | Optional framework hint; can infer language |
| `kind` | `any \| class \| interface \| function \| method \| type \| enum \| constructor \| annotation` | no | Stable public kind filter |
| `match` | `exact \| fuzzy` | no | Match mode; default `exact` |
| `path` | string | no | Workspace-relative or absolute scope |
| `limit` | integer | no | Default `50`, max `200` |

#### Data semantics

- `count`: compatibility field; equals `totalMatched`
- `returnedCount`: number of returned items
- `totalMatched`: full matched count before limit truncation
- `items[*].kind`: normalized to the stable public symbol kinds above

### `code.inspect_tree`

Inspect the workspace tree without reading file contents.

#### Input

| Field | Type | Required | Notes |
| --- | --- | --- | --- |
| `path` | string | no | Workspace-relative or absolute file/directory scope |
| `max_depth` | integer | no | Default `3`, max `20` |
| `extensions` | string[] | no | File extension filter; directories remain visible |
| `file_pattern` | string | no | Filename glob such as `*.py` |
| `include_stats` | boolean | no | Include stat metadata |
| `include_hidden` | boolean | no | Include hidden entries except hard-ignored directories |

#### Data semantics

- `entryCount`: number of returned entries
- `meta.counts.returnedCount`: same as `entryCount`
- `meta.counts.totalMatched`: present only when known; omitted as `null` when the safety cap prevents a full count

### `code.list_endpoints`

List backend endpoints and frontend routes in the workspace.

#### Input

| Field | Type | Required | Notes |
| --- | --- | --- | --- |
| `path` | string | no | Workspace-relative or absolute scope |
| `language` | `typescript \| javascript \| java` | no | Optional language filter |
| `framework` | `react-router \| spring` | no | Optional framework hint |
| `kind` | `any \| graphql \| rest \| route` | no | Stable public kind filter |
| `limit` | integer | no | Default `50`, max `200` |

#### Data semantics

- `totalCount`: full matched count before limit truncation
- `returnedCount`: number of returned items
- `counts.byKind|byLanguage|byFramework`: grouped counts across the full matched set

Backend-specific subtypes are normalized to the stable public kinds:

- GraphQL queries/mutations → `graphql`
- REST verbs and request mappings → `rest`
- React Router loaders/actions/layouts/resource routes/components → `route`

### `code.search_text`

Search plain text or regex patterns across workspace files.

#### Input

| Field | Type | Required | Notes |
| --- | --- | --- | --- |
| `query` | string | yes | Plain text or regex pattern |
| `path` | string | no | Workspace-relative or absolute scope |
| `language` | `typescript \| javascript \| java` | no | Optional language filter |
| `framework` | `react-router \| spring` | no | Optional framework hint |
| `include` | string | no | Additional include glob such as `*.tsx` or `src/**` |
| `regex` | boolean | no | Default `false` |
| `context` | integer | no | Default `1`, max `10` |
| `limit` | integer | no | Default `50`, max `200` |

#### Data semantics

- `fileCount`: returned matched file count
- `matchCount`: returned matched line count
- `totalFileCount`: full matched file count before limit truncation
- `totalMatchCount`: full matched line count before limit truncation

### `code.trace_symbol`

Trace a symbol forward from a known starting file.

#### Input

| Field | Type | Required | Notes |
| --- | --- | --- | --- |
| `path` | string | yes | Workspace-relative or absolute starting file path |
| `symbol` | string | yes | Symbol/function/method name to trace forward |
| `language` | `typescript \| javascript \| java` | no | Optional language hint |
| `framework` | `react-router \| spring` | no | Optional framework hint |

#### Data semantics

- `fileCount`: returned related file count
- `meta.counts.returnedCount`: same as `fileCount`

### `code.trace_callers`

Trace incoming callers from a known starting file.

#### Input

| Field | Type | Required | Notes |
| --- | --- | --- | --- |
| `path` | string | yes | Workspace-relative or absolute starting file path |
| `symbol` | string | yes | Symbol/function/method name to trace incoming callers for |
| `language` | `typescript \| javascript \| java` | no | Optional language hint |
| `framework` | `react-router \| spring` | no | Optional framework hint |
| `recursive` | boolean | no | Enable recursive reverse traversal |
| `max_depth` | integer | no | Only used with `recursive=true`; default `3`, min `1`, max `8` |

#### Data semantics

- `count`: compatibility field; equals `returnedCount`
- `returnedCount`: direct caller count returned in `items`
- Recursive mode is opt-in
- Recursive payloads may be safety-pruned; when that happens the tool returns `status: "partial"`, `RESULT_TRUNCATED`, and the recursive summary counts remain authoritative even if arrays were sliced

### Status semantics

- `ok`: request succeeded, including zero results
- `partial`: request succeeded with truncated or safety-pruned data
- `error`: request could not be completed

### Error semantics

Errors are always structured with:

- `code`: stable machine-readable code
- `message`: human-readable explanation
- `retryable`: whether retrying may help
- `suggestion`: concrete next step for the caller

### Coverage notes

- `code.find_symbol` is migrated through the TS -> Rust path and currently returns real definitions for Java, TypeScript, and JavaScript/JSX/TSX files
- `trace_symbol` and `trace_callers` remain on the compatibility path while their migrations are pending
- `search_text` is powered by ripgrep behind the normalized public contract
- Internal analyzer paths, commands, raw payloads, and local implementation details are intentionally NOT exposed in the public contract

## Sample usage

```json
{
  "name": "code.search_text",
  "arguments": {
    "query": "useLoaderData",
    "language": "typescript",
    "include": "app/routes/**/*.tsx",
    "context": 1,
    "limit": 20
  }
}
```
