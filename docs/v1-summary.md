# V1 Release Summary

## What shipped

V1 closed the first public contract for repository navigation with six tools:

- `code.find_symbol`
- `code.search_text`
- `code.trace_symbol`
- `code.trace_callers`
- `code.list_endpoints`
- `code.inspect_tree`

## V1 strengths

- stable normalized response envelope across all tools
- stable machine-readable error semantics
- safe workspace path resolution
- practical support for Java and TypeScript-family analysis where internal analyzers exist
- route and endpoint discovery under one public surface

## Known limitations and tradeoffs

### 1. V1 is navigation-only

This server does **not** edit files, run builds, or execute application-specific workflows.

### 2. The migration is hybrid right now

`inspect_tree`, `find_symbol`, and `list_endpoints` now run through the TypeScript runtime + Rust engine path.

`search_text`, `trace_symbol`, and `trace_callers` still use the legacy Python compatibility/oracle path while the migration remains in progress.

### 3. Search uses ripgrep

`code.search_text` depends on `rg` being available in the runtime environment.

### 4. Endpoint discovery is broader after the migration

`code.list_endpoints` now covers:

- React Router 7 routes for TypeScript/JavaScript
- Spring REST and GraphQL controllers for Java
- Python decorator / URL-pattern endpoint discovery
- Rust Actix-style attrs and async-graphql attrs

### 5. Safety caps are part of the contract

Large responses may be returned as `partial` with `RESULT_TRUNCATED` instead of attempting unbounded payloads.

### 6. Hard-ignore behavior is deliberate

`code.inspect_tree` never traverses hard-ignored directories such as `.git`, `node_modules`, `dist`, `build`, `target`, and similar heavy or unsafe folders, even when hidden entries are requested.

## Compatibility guidance for future releases

Keep these stable unless a new version is explicitly introduced:

- top-level envelope shape
- status semantics
- error code meanings
- path safety behavior
- count metadata semantics
- public tool names under `code.*`

## Good V2 directions

- migrate `code.search_text`, `code.trace_symbol`, and `code.trace_callers` off the Python path
- more explicit backend capability reporting
- richer recursive trace summaries
- stronger contract tests around backend error translation
