# Release Checklist

## Contract safety

- [ ] Confirm all public tool names remain correct under `code.*`
- [ ] Confirm envelope fields remain `tool`, `status`, `summary`, `data`, `errors`, `meta`
- [ ] Confirm `ok` / `partial` / `error` semantics still match the documented contract
- [ ] Confirm path normalization still returns `FILE_NOT_FOUND` and `PATH_OUTSIDE_WORKSPACE` correctly
- [ ] Confirm truncation still surfaces `RESULT_TRUNCATED` with accurate count metadata

## Tool coverage

- [ ] `code.find_symbol` discoverability and schema checked
- [ ] `code.search_text` discoverability and schema checked
- [ ] `code.trace_symbol` discoverability and schema checked
- [ ] `code.trace_callers` discoverability and schema checked
- [ ] `code.list_endpoints` discoverability and schema checked
- [ ] `code.inspect_tree` discoverability and schema checked

## Documentation

- [ ] README reflects current scope and supported tools
- [ ] `docs/overview.md` still matches product positioning
- [ ] `docs/v1-summary.md` reflects current shipped contract and limitations
- [ ] `docs/testing.md` matches the real test commands and layout

## Tests

- [ ] Run the full automated test suite
- [ ] Verify inspect-tree hard-ignore behavior
- [ ] Verify at least one truncation contract test
- [ ] Verify at least one path error contract test
- [ ] Verify MCP tool registration and schema discoverability

## Dependencies and runtime

- [ ] Confirm `pyproject.toml` test dependencies are current
- [ ] Confirm `uv.lock` is updated when dependencies change
- [ ] Confirm required runtime dependencies (`rg`, internal analyzers) are documented where relevant

## Before tagging a release

- [ ] Review known limitations and decide whether any require release notes
- [ ] Capture any contract changes explicitly instead of letting them ship silently
- [ ] If behavior changed, add or update tests BEFORE release notes are finalized
