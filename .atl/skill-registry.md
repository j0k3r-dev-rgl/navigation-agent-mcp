# Skill Registry

## Project conventions

- `AGENTS.md` (repo root): always use navigation tools first; prefer `read`, `glob`, and `apply_patch`; do not build after changes.
- `README.md`: Node 18+, `npx`-based MCP server, optional `rg` for text search.

## Project skill directories

- `skills/navigation-mcp`: workspace navigation, symbol lookup, endpoint listing, and tracing rules.

## Skill metadata

- `skills/navigation-mcp`
  - author: `j0k3r-dev-rgl`
  - version: `1.4.0`
  - notes: aligned with current public contract, removed docs/ dependency, and updated Go/Rust support snapshot

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
- Public MCP support is verified as strong for Java, TypeScript, and Rust; Go now works well for symbol lookup, search, trace flow, and trace callers, while endpoint detection remains limited in the current example.
