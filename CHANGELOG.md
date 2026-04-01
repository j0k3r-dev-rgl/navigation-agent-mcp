# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-04-01

### Features

* group callees by path and name to reduce noise

### Bug Fixes

* rename trace_symbol to trace_flow across codebase
* trace_flow now traces through ports to adapters using global index

### Documentation

* update README with complete capability matrix and trace_flow docs
* trace_flow works with TypeScript too

## [0.1.5] - 2026-03-31

### Bug Fixes

* gracefully handle permission denied errors during file scanning

## [0.1.3] - 2026-03-31

### Bug Fixes

* skip unreadable directories during workspace file walk

## [0.1.2] - 2026-03-31

### Bug Fixes

* skip unreadable files instead of failing in find_symbol, list_endpoints, trace_callers

## [0.1.1] - 2026-03-31

### Bug Fixes

* strip existing shebang before prepending in add-shebang.mjs

## [0.1.0] - 2026-03-31

### Features

* add npm publishing pipeline with cross-platform binary distribution
* complete code tools migration to TypeScript and Rust
* migrate list_endpoints to TypeScript and Rust
* migrate MCP foundation and find_symbol to TypeScript and Rust
