# Skill Registry

## Project conventions

- `AGENTS.md` (repo root): always use navigation tools first; prefer `read`, `glob`, and `apply_patch`; do not build after changes.
- `README.md`: Node 18+, `npx`-based MCP server, optional `rg` for text search.
- `docs/development.md`: npm-first runtime, Rust engine, test commands, and `NAVIGATION_MCP_RUST_ENGINE_CMD` override.
- `docs/overview.md`: public `workspace.*` contract and normalized response envelope.

## Project skill directories

- `skills/navigation-mcp`: workspace navigation, symbol lookup, endpoint listing, and tracing rules.

## Available skills in this environment

- `sdd-init`
- `sdd-explore`
- `sdd-propose`
- `sdd-spec`
- `sdd-design`
- `sdd-tasks`
- `sdd-apply`
- `sdd-verify`
- `sdd-archive`
- `navigation-mcp`
- `branch-pr`
- `issue-creation`
- `go-testing`
- `java-spring`
- `java-spring-mongo`
- `react-router-7`
- `skill-creator`
- `judgment-day`
- `engram-pending-tasks`

## Notes

- This repo has no detected Biome, ESLint, or Prettier config.
- Rust uses `clippy` allowances in source, but no repo-level lint command was detected.
