# Skill Registry

## Project conventions

- `AGENTS.md` (repo root): always use navigation tools first; prefer `read`, `glob`, and `apply_patch`; do not build after changes.
- `README.md`: Node 18+, `npx`-based MCP server, optional `rg` for text search.
- `docs/development.md`: npm-first runtime, Rust engine, test commands, and `NAVIGATION_MCP_RUST_ENGINE_CMD` override.
- `docs/overview.md`: public `code.*` contract, compatibility matrix, and normalized response envelope.

## Project skill directories

- `skills/navigation-mcp`: workspace navigation, symbol lookup, endpoint listing, and tracing rules.

## Skill metadata

- `skills/navigation-mcp`
  - author: `j0k3r-dev-rgl`
  - version: `1.3.0`
  - notes: aligned with current public contract and real support snapshot

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
- Public MCP support currently verified as strong for Java and TypeScript, strong-but-not-complete for Rust, and not yet ready as public support for Go.
