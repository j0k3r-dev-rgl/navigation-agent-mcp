---
name: navigation-mcp
description: "Trigger: MCP navigation, code discovery, symbol lookup, trace_flow, trace_callers, endpoints. Use navigation-agent MCP before reading workspace code."
license: Apache-2.0
compatibility: opencode
metadata:
  author: j0k3r-dev-rgl
  version: "1.7.0"
---

## Activation Contract

Use this skill for workspace-only code discovery: locating files, symbols, routes, endpoints, text matches, caller impact, or downstream flow before editing. Do not use it for web search, external repositories, parent directories, arbitrary filesystem access, or reading full file contents.

## Hard Rules

1. If discovery is needed, use navigation first; do not open multiple files cold.
2. If the exact file is already known and no discovery is needed, `read` is allowed.
3. Canonical tool names are `code.inspect_tree`, `code.find_symbol`, `code.list_endpoints`, `code.search_text`, `code.trace_flow`, and `code.trace_callers`.
4. Clients may expose aliases such as `navigation-agent_code_find_symbol`; use the available alias but preserve the canonical `code.*` semantics.
5. Supported language filters: `typescript`, `javascript`, `go`, `java`, `php`, `python`, `rust`, `csharp`.
6. Supported framework filters: `react-router`, `spring`.
7. Use `snake_case` parameters (`max_depth`, `include_hidden`, `file_pattern`, `recursive`).
8. Treat navigation output as a scope reducer, not file content. Read only returned files that matter.
9. Use `glob`, `bash grep`, or `bash find` only when navigation is unavailable, unsupported for the task, or returned no useful result after narrowing.

## Decision Gates

| Need | Use first | Then |
| --- | --- | --- |
| Understand an unknown module | `code.inspect_tree` | Read selected paths only |
| Find a class/function/method/type | `code.find_symbol` | Trace or read returned path |
| Audit REST/GraphQL/routes | `code.list_endpoints` | Trace or read entrypoints |
| Find imports/decorators/text | `code.search_text` | Prioritize `topFiles`, then read |
| Know what a symbol calls | `code.find_symbol` | `code.trace_flow` with `items[].path` |
| Know who calls a symbol | `code.find_symbol` | `code.trace_callers` with `recursive: true` for shared APIs |

## Execution Steps

1. Identify the narrowest known scope: `path`, `language`, and `framework` when available.
2. Start with the most specific navigation tool from the decision table.
3. Chain tool outputs: pass `find_symbol.items[].path` as `path` to `trace_flow` or `trace_callers`.
4. Remember direction: `trace_flow` is downstream behavior; `trace_callers` is upstream impact.
5. If results are empty or `truncated: true`, narrow `path`, `limit`, `kind`, or `language` before falling back.
6. Read only the small set of files needed to answer or edit safely.

## Output Contract

When reporting back, mention the navigation path used, important resolved files, any fallback, and limitations such as unsupported framework detection or zero endpoint results.

## Anti-Patterns

- Reading broad directories before navigation.
- Using text search when a symbol trace would answer better.
- Calling `trace_flow` to find callers or `trace_callers` to follow behavior.
- Assuming this MCP can inspect the web or external repos.
- Treating a zero-result navigation response as proof without a scoped fallback.

## References

- `README.md` — public contract, compatibility matrix, and agent usage guide.
