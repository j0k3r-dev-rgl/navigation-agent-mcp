# Changelog

All notable changes to this project will be documented in this file.

## [0.5.0](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/compare/v0.4.0...v0.5.0) (2026-04-03)


### Features

* streamline search text output for agents ([5478c07](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/commit/5478c073227ed8569e3ca419a6c03ec55a60d206))

## [0.4.0](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/compare/v0.3.5...v0.4.0) (2026-04-03)


### Features

* improve go tracing and agent analysis output ([62acdd1](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/commit/62acdd1df83cb77fb3fd88a9803e49db428d0ba2))


### Bug Fixes

* improve go trace flow recursion ([8ddffdc](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/commit/8ddffdcd74031178596ca34a41ff8f6f8ef76ce8))
* simplify internal rust analyzer contracts ([6d7971f](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/commit/6d7971fe990bde9fd4d3ce782fa03920506ced3e))

## [0.3.5](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/compare/v0.3.4...v0.3.5) (2026-04-02)


### Bug Fixes

* improve analyzer support and sync release baseline ([1833220](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/commit/18332203946ab534bec01e7824e8c09a6965705d))

## [0.3.4](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/compare/v0.3.3...v0.3.4) (2026-04-02)


### Bug Fixes

* align release versions to 0.3.4 ([9374ffa](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/commit/9374ffafc75bafad0a81e384f51fb064c49becad))
* align release-please to 0.3.4 ([0cc5a74](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/commit/0cc5a7479b25cd49681829619903d42516fd9dcc))
* align release-please version updates ([2280755](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/commit/2280755cc25c64a7848478d4a020ddef5ed431d1))
* anchor release-please changelog to v0.3.3 ([67e3f30](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/commit/67e3f307fb0f3d3b24f05905bbcae9456c47e56a))
* match release-please tags to v format ([586de0c](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/commit/586de0c6a6ae0fbc7217dd9fcfa90c10a77d54be))
* remove temporary release-please overrides ([311f99f](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/commit/311f99ff42a29e5908728ad41934722ea6c738f7))
* restore release manifest to 0.3.3 ([1829441](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/commit/18294414204a2e125cf481beb19ef1d09f72a18d))

## [0.3.0](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/compare/navigation-agent-mcp-v0.2.1...navigation-agent-mcp-v0.3.0) (2026-04-01)


### Features

* add npm publishing pipeline with cross-platform binary distribution ([0f053de](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/commit/0f053de840f391e6a865b65ceff6786d9bf60064))
* complete code tools migration to TS and Rust ([269dfea](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/commit/269dfeabfa96293ede21e4a9b136e47301968647))
* group callees by path and name to reduce noise ([81d55f1](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/commit/81d55f1f8092bdcd53f7ff49f2288ca8a7afc66b))
* migrate list endpoints to TS and Rust ([a3200cf](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/commit/a3200cffb91b6c6436d14725b1db9c43b3d037a7))
* migrate MCP foundation and find_symbol to TS and Rust ([f7d9362](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/commit/f7d9362255707d73566879e70b3719e934bec491))


### Bug Fixes

* broken export of traceFlowService in index.ts ([0413f8d](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/commit/0413f8d1777568804d1b2f515b87085a182859b0))
* force cargo target-dir to repo root to fix artifact path ([214e021](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/commit/214e021f7f72fa3f51d24a40f8e9a688a96600b2))
* gracefully handle permission denied errors during file scanning ([ecebf16](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/commit/ecebf1678359c36208f7f1595b4020cd38643509))
* ignore platform checks in npm ci and opt into Node 24 actions ([63f92c0](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/commit/63f92c0d09034bcf73418d717dab9029478fb8a0))
* remove incorrect type casts in registerCodeTools handlers ([f3e5412](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/commit/f3e541215297cd115f609b8a6201be6012fe962c))
* remove platform packages from workspaces to avoid EBADPLATFORM in CI ([691836d](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/commit/691836df942bd1b4f6597dac7244240481b7b180))
* rename trace_symbol to trace_flow across codebase ([fb43959](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/commit/fb439596b48ea83081893ab6f63d168711ae3eaf))
* replace deprecated macos-13 runner with macos-latest ([b59e34c](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/commit/b59e34c7f808817ef50d8f5757c2a29a979033e8))
* resolve tsc build errors — rewriteRelativeImportExtensions and type casts ([df8d5e1](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/commit/df8d5e1fe80f691cb2e0c56d9f8a17509756846b))
* skip unreadable directories during workspace file walk ([30009d9](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/commit/30009d9c0e38dea0d40625ee7ac58bb07cd7e362))
* skip unreadable files instead of failing in find_symbol, list_endpoints, trace_callers ([a057ff7](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/commit/a057ff77a5ae5745e9bf80885c8d181e6d354572))
* strip existing shebang before prepending in add-shebang.mjs ([737673f](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/commit/737673fd3ed4d4283fe7bd700d123b7d27a908dd))
* trace_flow now traces through ports to adapters using global index ([c0d032b](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/commit/c0d032ba52aea182429eec450decee01fd102966))
* use ./ prefix in npm publish paths to avoid GitHub shorthand interpretation ([ff104e3](https://github.com/j0k3r-dev-rgl/navigation-agent-mcp/commit/ff104e3b97616cea22c316ccf248f6a982e14ccc))

## [0.2.1] - 2026-04-01

### Bug Fixes

* fix broken export of traceFlowService in index.ts

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
