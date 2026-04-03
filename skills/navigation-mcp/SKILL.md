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
  author: j0k3r-dev-rgl
  version: "1.4.0"
---

## When to Use

- Investigating an unfamiliar codebase or feature
- Locating files, symbols, callers, routes, endpoints, or relevant text matches
- Scoping the minimum set of files to read before editing
- Performing impact analysis before changing shared code

## Critical Rules

1. Always start with a navigation tool — never open files cold.
2. Use the most specific tool first. Broad tools (search_text) are fallbacks, not first choices.
3. After navigation narrows the scope, read only the files that were returned.
4. Fall back to `read` / `glob` only when navigation returns no match or the task is outside its scope.
5. Never use `bash grep` or `bash find` for code search while navigation tools are available.
6. The stable public MCP contract is `code.*`, not `workspace.*`.
7. Current public languages are `typescript`, `javascript`, `go`, `java`, `python`, and `rust`.
8. Go is now usable through the public contract for `find_symbol`, `search_text`, `trace_flow`, and `trace_callers`, but `list_endpoints` is still limited on the current example app.

## Tool Decision Guide

| What you need | Tool to use first | Fallback |
| --- | --- | --- |
| Orient in an unknown module or directory | `navigation_code_inspect_tree` | `glob` |
| Find where a class, function, or type is defined | `navigation_code_find_symbol` | `glob`, `read` |
| Find all usages of a pattern, annotation, or import | `navigation_code_search_text` | `read` |
| Get the full API or route surface | `navigation_code_list_endpoints` | `read` |
| Follow what a symbol calls or depends on (forward) | `navigation_code_trace_flow` | `read` |
| Find what calls a symbol — impact before a change (backward) | `navigation_code_trace_callers` | `read` |

## Semantic Distinction: trace_flow vs trace_callers

These two tools are opposites. Confusing them produces wrong results.

- **`navigation_code_trace_flow`** — traces **forward** (downstream).
  Use it when you want to know: _"What does this function call? What files does it touch?"_
  Example: you have a controller and want to follow the full call chain into use cases, adapters, and repositories.

- **`navigation_code_trace_callers`** — traces **backward** (upstream).
  Use it when you want to know: _"Who calls this function? What will break if I change it?"_
  Example: you are about to rename a shared utility and need to know all callers before touching it.

## Parameter Handoff Between Tools

Navigation tools chain together. The output of one tool feeds the input of the next.

**`find_symbol` → `trace_flow` / `trace_callers`**

`find_symbol` returns `items[].path` — the file where the symbol is defined.
Pass that `path` directly as the `path` parameter of `trace_flow` or `trace_callers`.

```
find_symbol(symbol: "CreateTitularUseCase", language: "java", kind: "class")
  → returns items[0].path = "modules/titular/infrastructure/use_cases/command/CreateTitularUseCase.java"

trace_flow(
  path: "modules/titular/infrastructure/use_cases/command/CreateTitularUseCase.java",
  symbol: "CreateTitularUseCase",
  language: "java"
)
```

**`inspect_tree` → `find_symbol` / `search_text`**

`inspect_tree` returns directory paths. Use those paths to scope subsequent searches.

```
inspect_tree(path: "modules/titular", max_depth: 3)
  → confirms "modules/titular/infrastructure/web/graphql/" exists

find_symbol(symbol: "getTitularById", kind: "method", path: "modules/titular")
```

## Tool Reference

### navigation_code_inspect_tree

Inspect the workspace file tree without reading file contents.

| Parameter | Type | Default | Description |
| --- | --- | --- | --- |
| `path` | `string \| null` | `null` | Workspace-relative or absolute scope. `null` = workspace root |
| `max_depth` | `integer` | `3` | Max depth from the scope root (0–20) |
| `extensions` | `string[] \| null` | `null` | Filter by extensions, e.g. `['.ts', '.tsx']`. Directories remain visible |
| `file_pattern` | `string \| null` | `null` | Filename glob, e.g. `'*.test.ts'` |
| `include_stats` | `boolean` | `false` | Include file size, modified time, symlink info |
| `include_hidden` | `boolean` | `false` | Include hidden files/directories |

### navigation_code_find_symbol

Locate symbol definitions by name. Returns `items[].path` and `items[].line`.

| Parameter | Type | Default | Description |
| --- | --- | --- | --- |
| `symbol` | `string` | **required** | Symbol name to find |
| `language` | `string \| null` | `null` | `typescript`, `javascript`, `java`, `python`, `rust` |
| `framework` | `string \| null` | `null` | `react-router`, `spring` |
| `kind` | `string` | `"any"` | `any`, `class`, `interface`, `function`, `method`, `type`, `enum`, `constructor`, `annotation` |
| `match` | `string` | `"exact"` | `exact` or `fuzzy` |
| `path` | `string \| null` | `null` | Limit search to a specific path |
| `limit` | `integer` | `50` | Max results (1–200) |

### navigation_code_search_text

Search text or regex patterns across the workspace.

| Parameter | Type | Default | Description |
| --- | --- | --- | --- |
| `query` | `string` | **required** | Search text or regex |
| `path` | `string \| null` | `null` | Limit to a path |
| `language` | `string \| null` | `null` | Filter by language |
| `framework` | `string \| null` | `null` | Filter by framework |
| `include` | `string \| null` | `null` | File glob filter, e.g. `'*.service.ts'` |
| `regex` | `boolean` | `false` | Treat query as a regular expression |
| `context` | `integer` | `1` | Lines of context before/after each match (0–10) |
| `limit` | `integer` | `50` | Max files returned (1–200) |

### navigation_code_list_endpoints

List backend endpoints and frontend routes in the workspace.

| Parameter | Type | Default | Description |
| --- | --- | --- | --- |
| `path` | `string \| null` | `null` | Limit to a path |
| `language` | `string \| null` | `null` | `typescript`, `javascript`, `java`, `python`, `rust` |
| `framework` | `string \| null` | `null` | `react-router`, `spring` |
| `kind` | `string` | `"any"` | `any`, `graphql`, `rest`, `route` |
| `limit` | `integer` | `50` | Max results (1–200) |

### navigation_code_trace_flow

Trace execution **forward** from a symbol — what it calls, what files it touches.
`path` must be the file where the symbol is defined. Get it from `find_symbol` first.

| Parameter | Type | Default | Description |
| --- | --- | --- | --- |
| `path` | `string` | **required** | File where the symbol is defined (must exist in workspace) |
| `symbol` | `string` | **required** | Symbol name to trace forward |
| `language` | `string \| null` | `null` | `typescript`, `javascript`, `java`, `python`, `rust` |
| `framework` | `string \| null` | `null` | `react-router`, `spring` |

### navigation_code_trace_callers

Trace incoming callers **backward** — who calls this symbol.
`path` must be the file where the symbol is defined. Get it from `find_symbol` first.
Use `recursive: true` for full impact analysis before renaming shared code.

| Parameter | Type | Default | Description |
| --- | --- | --- | --- |
| `path` | `string` | **required** | File where the symbol is defined |
| `symbol` | `string` | **required** | Symbol name to trace callers for |
| `language` | `string \| null` | `null` | `typescript`, `javascript`, `java`, `python`, `rust` |
| `framework` | `string \| null` | `null` | `react-router`, `spring` |
| `recursive` | `boolean` | `false` | Enable recursive reverse-traversal up the call tree |
| `max_depth` | `integer \| null` | `null` | Max recursion depth (1–8). Only applies when `recursive: true` |

## Workflows

### Orient in a module

```
Goal: understand a module's structure before reading any file

1. navigation_code_inspect_tree(path: "modules/titular", max_depth: 3)
   → see directories and files without opening them

2. read() only the specific files that matter
```

### Find a symbol and trace its flow forward

```
Goal: follow the call chain from a controller into the domain

1. navigation_code_find_symbol(symbol: "CreateTitularUseCase", language: "java", kind: "class")
   → returns items[0].path = "modules/titular/infrastructure/..."

2. navigation_code_trace_flow(
     path: items[0].path,
     symbol: "CreateTitularUseCase",
     language: "java"
   )
   → returns all files and callees this symbol reaches

3. read() only the files returned by trace_flow
```

### Impact analysis before changing shared code

```
Goal: know who calls a utility before renaming or changing its signature

1. navigation_code_find_symbol(symbol: "MongoIdUtils", language: "java", kind: "class")
   → returns items[0].path

2. navigation_code_trace_callers(
     path: items[0].path,
     symbol: "MongoIdUtils",
     language: "java",
     recursive: true,
     max_depth: 3
   )
   → returns all callers and their paths

3. read() only the impacted files before making the change
```

### Audit the API surface

```
Goal: get a full map of REST or GraphQL endpoints before adding a new one

1. navigation_code_list_endpoints(framework: "spring", kind: "rest")
   → full index of REST controllers without reading files

2. navigation_code_list_endpoints(framework: "spring", kind: "graphql")
   → full index of GraphQL resolvers
```

### Find all usages of a pattern

```
Goal: find every file that imports or uses a specific decorator or annotation

1. navigation_code_search_text(
     query: "@QueryMapping",
     language: "java",
     context: 2
   )
   → returns file paths and match context

2. read() only the relevant matches
```

## Fallback Rules

- Use `read` / `glob` only after a navigation tool returned no results or the path is already known exactly.
- Use `bash` search tools only when navigation is genuinely unavailable or broken.
- When a navigation tool returns partial results (`truncated: true`), narrow the scope with `path` or `limit` before falling back.
- When validating support claims, prefer real-project checks over toy examples and contrast the output against real source files.

## Real Support Snapshot

Verified during the latest project sync:

| Capability | Java | TypeScript / JavaScript | Rust | Go |
| --- | --- | --- | --- | --- |
| `inspect_tree` | ✅ | ✅ | ✅ | ✅ |
| `find_symbol` | ✅ | ✅ | ✅ | ✅ |
| `search_text` | ✅ | ✅ | ✅ | ✅ |
| `list_endpoints` | ✅ | ✅ | ⚠️ target-dependent | ⚠️ limited in current example |
| `trace_flow` | ✅ | ✅ | ✅ | ✅ |
| `trace_callers` | ✅ | ✅ | ✅ with qualified symbols | ✅ |

Use this table as guidance for expectations, not as a substitute for real validation.

## Anti-Patterns

- Opening files before running any navigation tool
- Using `bash grep` or `bash find` when `search_text` or `find_symbol` would answer directly
- Calling `trace_flow` or `trace_callers` without getting the `path` from `find_symbol` first
- Using `trace_flow` to find callers (wrong direction — use `trace_callers`)
- Using `trace_callers` to follow a feature forward (wrong direction — use `trace_flow`)
- Passing `path: null` when a scoped path is already known
