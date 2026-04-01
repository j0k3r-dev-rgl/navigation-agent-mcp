---
name: navigation-mcp
description: >
  Prioritize the navigation MCP for structural code discovery, impact analysis,
  and scoped reading before falling back only to targeted reads, globbing, or bash.
  Trigger: When the task requires finding symbols, tracing flows, listing routes or endpoints,
  inspecting code structure, or searching a workspace efficiently.
license: Apache-2.0
compatibility: opencode
metadata:
  author: gentleman-programming
  version: "1.1.0"
---

## When to Use

- Investigating an unfamiliar codebase or feature
- Locating files, symbols, callers, routes, endpoints, or relevant text matches
- Scoping the minimum set of files to read before editing
- Performing impact analysis before changing shared code

## Critical Patterns

1. Prefer navigation MCP tools first for code discovery.
2. Use the most specific navigation tool available before broader search.
3. Read only the minimum file set after navigation narrows the scope.
4. Fall back only to `read` / `glob` when navigation returns no relevant match or the task is outside its scope.
5. Use `bash` for code search only as a last resort.

## Tool Selection Order

| Need | Use first | Fallback |
| --- | --- | --- |
| Inspect tree / orient in a module | `navigation_code_inspect_tree` | `glob` |
| Find symbol definitions | `navigation_code_find_symbol` | `read`, `glob` |
| Search text across code | `navigation_code_search_text` | `read` |
| List routes / endpoints | `navigation_code_list_endpoints` | `read` |
| Trace feature flow | `navigation_code_trace_flow` | `read` |
| Find incoming callers | `navigation_code_trace_callers` | `read` |

## Tool Reference

### navigation_code_inspect_tree

Inspect the workspace file tree without reading file contents.

| Parameter | Type | Default | Description |
| --- | --- | --- | --- |
| `path` | `string \| null` | `null` | Optional workspace-relative or absolute file/directory scope. `null` = workspace root |
| `max_depth` | `integer` | `3` | Maximum depth relative to the resolved scope root (0-20) |
| `extensions` | `string[] \| null` | `null` | Filter by file extensions, e.g. `['.ts', '.tsx']`. Directories remain visible |
| `file_pattern` | `string \| null` | `null` | Glob pattern, e.g. `'*.test.ts'` |
| `include_stats` | `boolean` | `false` | Include size, modified time, and symlink metadata |
| `include_hidden` | `boolean` | `false` | Include hidden files/directories |

### navigation_code_find_symbol

Locate symbol definitions in the workspace.

| Parameter | Type | Default | Description |
| --- | --- | --- | --- |
| `symbol` | `string` | **required** | Symbol name to search for |
| `language` | `string \| null` | `null` | Filter by language: `typescript`, `javascript`, `java`, `python`, `rust` |
| `framework` | `string \| null` | `null` | Filter by framework: `react-router`, `spring` |
| `kind` | `string` | `"any"` | Symbol kind: `any`, `class`, `interface`, `function`, `method`, `type`, `enum`, `constructor`, `annotation` |
| `match` | `string` | `"exact"` | Match mode: `exact` or `fuzzy` |
| `path` | `string \| null` | `null` | Limit search to a specific path |
| `limit` | `integer` | `50` | Maximum results (1-200) |

### navigation_code_search_text

Search text or regex patterns across the workspace.

| Parameter | Type | Default | Description |
| --- | --- | --- | --- |
| `query` | `string` | **required** | Search text or regex pattern |
| `path` | `string \| null` | `null` | Limit search to a specific path |
| `language` | `string \| null` | `null` | Filter by language |
| `framework` | `string \| null` | `null` | Filter by framework |
| `include` | `string \| null` | `null` | File pattern to include, e.g. `'*.service.ts'` |
| `regex` | `boolean` | `false` | Treat query as regular expression |
| `context` | `integer` | `1` | Lines of context before/after match (0-10) |
| `limit` | `integer` | `50` | Maximum files to return (1-200) |

### navigation_code_list_endpoints

List backend endpoints and frontend routes.

| Parameter | Type | Default | Description |
| --- | --- | --- | --- |
| `path` | `string \| null` | `null` | Limit to a specific path |
| `language` | `string \| null` | `null` | Filter: `typescript`, `javascript`, `java`, `python`, `rust` |
| `framework` | `string \| null` | `null` | Filter: `react-router`, `spring` |
| `kind` | `string` | `"any"` | Endpoint kind: `any`, `graphql`, `rest`, `route` |
| `limit` | `integer` | `50` | Maximum results (1-200) |

### navigation_code_trace_flow

Trace execution flow forward from a starting file and symbol to related workspace files.

| Parameter | Type | Default | Description |
| --- | --- | --- | --- |
| `path` | `string` | **required** | Starting file path (must exist in workspace) |
| `symbol` | `string` | **required** | Symbol name to trace |
| `language` | `string \| null` | `null` | Filter by language |
| `framework` | `string \| null` | `null` | Filter by framework |

### navigation_code_trace_callers

Trace incoming callers for a symbol using reverse traversal.

| Parameter | Type | Default | Description |
| --- | --- | --- | --- |
| `path` | `string` | **required** | Starting file path |
| `symbol` | `string` | **required** | Symbol name to trace callers for |
| `language` | `string \| null` | `null` | Filter by language |
| `framework` | `string \| null` | `null` | Filter by framework |
| `recursive` | `boolean` | `false` | Enable recursive reverse-traversal |
| `max_depth` | `integer \| null` | `null` | Max recursion depth for recursive mode (1-8) |

## Workflow

### 1. Start structurally

```
Need to understand a module's structure?
1. navigation_code_inspect_tree with path and max_depth
2. read only the files that matter
```

### 2. Find and trace symbols

```
Need controller/use-case flow?
1. navigation_code_find_symbol to locate the entry point
2. navigation_code_trace_flow to follow the chain
3. read only the traced files you truly need
```

```
Need callers of a shared function before changing it?
1. navigation_code_trace_callers
2. read only the impacted callers
```

### 3. Search with precision

```
Need to find all usages of a pattern?
1. navigation_code_search_text with path + language filters
2. Use regex: true for complex patterns
```

### 4. Filter endpoints

```
Need to audit API surface?
1. navigation_code_list_endpoints with kind: "rest" or "graphql"
2. Filter by framework: "spring" or "react-router"
```

## Good Patterns

```text
Need usages of a shared helper?
1. navigation_code_trace_callers (path + symbol)
2. navigation_code_search_text if you need text-level confirmation
3. read only the impacted callers
```

```text
Need to understand a Java Spring module?
1. navigation_code_inspect_tree with path: "modules/[name]"
2. navigation_code_list_endpoints with framework: "spring", kind: "rest"
3. navigation_code_find_symbol with language: "java", kind: "class"
```

## Anti-Patterns

- Opening many files before tracing the feature
- Using bash grep/find first when navigation can answer directly
- Mixing broad reads with no narrowing step
- Treating navigation as optional when the task is code discovery
- Using `path: null` (workspace root) when a scoped path would be faster

## Review Checklist

- [ ] Started with a navigation MCP tool when the task involved code discovery
- [ ] Chose the most specific navigation tool first
- [ ] Used path scoping to limit search space when possible
- [ ] Read only the minimum necessary files after narrowing scope
- [ ] Used fallback tools only when navigation was insufficient
