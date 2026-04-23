# Skill Registry

**Delegator use only.** Any agent that launches sub-agents reads this registry to resolve compact rules, then injects them directly into sub-agent prompts. Sub-agents do NOT read this registry or individual SKILL.md files.

See `_shared/skill-resolver.md` for the full resolution protocol.

## User Skills

| Trigger | Skill | Path |
|---------|-------|------|
| When the task requires finding symbols, tracing flows, listing routes or endpoints, inspecting code structure, or searching the current workspace efficiently. | navigation-mcp | /home/j0k3r/.config/opencode/skills/navigation-mcp/SKILL.md |
| When user says "judgment day", "judgment-day", "review adversarial", "dual review", "doble review", "juzgar", "que lo juzguen". | judgment-day | /home/j0k3r/.config/opencode/skills/judgment-day/SKILL.md |
| When user mentions "pending", "tech-debt", "TODO", "pendiente", "deuda técnica", or asks about pending tasks. | engram-pending-tasks | /home/j0k3r/.config/opencode/skills/engram-pending-tasks/SKILL.md |
| When editing, adding, or refactoring Rust code in `crates/navigation-engine`. | rust | /home/j0k3r/navigation-agent-mcp/.opencode/skills/rust/SKILL.md |
| When adding, updating, or validating Rust tests for the navigation engine, or when modifying analyzer or capability logic. | test-rust | /home/j0k3r/navigation-agent-mcp/.opencode/skills/test-rust/SKILL.md |
| When verifying changes to MCP tools, language analyzers, or the public contract before finalizing an implementation. | test-runtime | /home/j0k3r/navigation-agent-mcp/.opencode/skills/test-runtime/SKILL.md |
| When the task involves packages/mcp-server/src/contracts/public, packages/contract-tests, code.* tool names, or README.md compatibility updates. | contracts | /home/j0k3r/navigation-agent-mcp/.opencode/skills/contracts/SKILL.md |
| When the task involves packages/mcp-server, tool registration, contracts, or server-side TypeScript logic. | mcp-server | /home/j0k3r/navigation-agent-mcp/.opencode/skills/mcp-server/SKILL.md |

## Compact Rules

Pre-digested rules per skill. Delegators copy matching blocks into sub-agent prompts as `## Project Standards (auto-resolved)`.

### navigation-mcp
- Use navigation tools first for workspace discovery; never open files cold when structure can be resolved first.
- Prefer the most specific tool: `find_symbol` / `trace_flow` / `trace_callers` before `search_text`; `search_text` before fallback reads.
- Read only files returned by navigation; use `read`/`glob` only when navigation cannot answer.
- Public contract uses `code.*`, not `workspace.*`.
- `trace_flow` is downstream; `trace_callers` is upstream; do not confuse them.
- `find_symbol.items[].path` feeds directly into `trace_flow` / `trace_callers`.

### judgment-day
- Resolve relevant compact rules from the registry before launching judges.
- Launch two blind judges in parallel via delegation; neither knows about the other.
- Synthesize findings yourself into confirmed/suspect/contradiction buckets.
- Ask user before fixing confirmed issues after round 1.
- Re-judge only for confirmed CRITICALs after round 1; fix lesser confirmed issues without re-judge.
- If no registry exists, warn and proceed without project-specific standards.

### engram-pending-tasks
- Treat pending/TODO/tech-debt requests as Engram memory tasks first, not repo scans.
- Use deterministic topic keys: `pending/{slug}` and `pending-index/{project}`.
- Normalize synonyms but keep exact status/priority fields consistent.
- Always update the `Updated` timestamp when task state changes.
- Maintain the project pending index as the first lookup surface.
- Do not scan source comments for TODO/FIXME unless user explicitly asks.

### rust
- Keep Rust engine changes inside `crates/navigation-engine` unless explicitly asked otherwise.
- Match existing analyzer layout: `mod.rs`, optional/shared `common.rs`, and capability files when logic grows.
- Register new analyzers in both `src/analyzers/mod.rs` and `src/analyzers/registry.rs`.
- Preserve qualified Rust method symbols as `Owner::method`.
- Keep analyzer behavior aligned with `LanguageAnalyzer`; unsupported features return empty results, not panics.
- Do not run `cargo build` or `cargo test` unless user explicitly asks.

### test-rust
- Keep Rust tests in `crates/navigation-engine/tests/` following the current repo pattern.
- Use `analyzers_<language>.rs` for analyzer tests and `capabilities_<feature>.rs` for capability tests.
- Analyzer tests use raw source snippets and direct analyzer calls.
- Capability tests use `tempfile::tempdir()` plus real filesystem writes.
- Keep tests synchronous and scoped to the changed behavior.
- Use qualified Rust symbols (`Owner::method`) in trace/find-symbol assertions.

### test-runtime
- Prefer targeted runtime validation over broad builds or full-suite runs.
- Reuse or add manual scripts under `test-runtime/test-*.mjs`; do not save new runtime scripts in repo root.
- Use examples/ for language-specific validation when a matching fixture exists.
- Use `npm run mcp-server:dev` or direct `tsx` invocation for source-based MCP checks.
- Use `npm run contract-tests` when public contract behavior changes.
- Do not use `scripts/*.mjs` for normal runtime validation; those are packaging/release helpers.

### contracts
- `packages/mcp-server/src/contracts/public/code.ts` is the source of truth for public `code.*` tools.
- Never rename/remove exposed `code.*` tools; keep public params in `snake_case`.
- Preserve `ResponseEnvelope` top-level shape; put new output inside `data`.
- Contract changes start in the contract file before service implementation.
- Update README compatibility matrix when language/framework support changes.
- Run/update contract coverage in `packages/contract-tests` and `packages/mcp-server/test/contract/` when public behavior changes.

### mcp-server
- Work contract-first: update public contract types/normalizers before service logic when I/O changes.
- Tool changes span contract file, service, `registerCodeTools.ts`, and `createMcpServer.ts`.
- Keep bootstrap/transport, contracts, services, and engine bridge separated.
- Use package-local `test/unit/` and `test/contract/`, plus `packages/contract-tests` when the external contract changes.
- Reuse `test-runtime/` scripts for observable MCP runtime validation.
- Prefer `npm run mcp-server:check` / `mcp-server:test`; avoid builds unless explicitly requested.

## Project Conventions

| File | Path | Notes |
|------|------|-------|
| .gitignore | /home/j0k3r/navigation-agent-mcp/.gitignore | `.atl/` and `.opencode/` are ignored; no `AGENTS.md` was found in this project. |

Read the convention files listed above for project-specific patterns and rules. All referenced paths have been extracted — no need to read index files to discover more.
