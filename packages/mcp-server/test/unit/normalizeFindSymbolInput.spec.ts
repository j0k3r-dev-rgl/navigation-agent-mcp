import assert from "node:assert/strict";
import test from "node:test";

import { normalizeFindSymbolInput } from "../../src/contracts/public/code.js";

test("normalizeFindSymbolInput trims symbol and path and applies defaults", () => {
  const normalized = normalizeFindSymbolInput({
    symbol: "  loader  ",
    path: "  src/routes  ",
  });

  assert.equal(normalized.ok, true);
  if (!normalized.ok) {
    return;
  }

  assert.deepEqual(normalized.value, {
    symbol: "loader",
    language: null,
    framework: null,
    kind: "any",
    match: "exact",
    path: "src/routes",
    limit: 50,
  });
});

test("normalizeFindSymbolInput accepts python and rust without changing defaults", () => {
  const normalized = normalizeFindSymbolInput({
    symbol: "  fetch_users  ",
    language: "python",
  });

  assert.equal(normalized.ok, true);
  if (!normalized.ok) {
    return;
  }

  assert.deepEqual(normalized.value, {
    symbol: "fetch_users",
    language: "python",
    framework: null,
    kind: "any",
    match: "exact",
    path: null,
    limit: 50,
  });

  const rustNormalized = normalizeFindSymbolInput({
    symbol: "  AnalyzerRegistry  ",
    language: "rust",
  });

  assert.equal(rustNormalized.ok, true);
  if (!rustNormalized.ok) {
    return;
  }

  assert.deepEqual(rustNormalized.value, {
    symbol: "AnalyzerRegistry",
    language: "rust",
    framework: null,
    kind: "any",
    match: "exact",
    path: null,
    limit: 50,
  });
});

test("normalizeFindSymbolInput rejects missing symbol and invalid limit", () => {
  const normalized = normalizeFindSymbolInput({ symbol: "   ", limit: 500 });
  assert.equal(normalized.ok, false);
  if (normalized.ok) {
    return;
  }

  assert.deepEqual(
    normalized.issues.map((issue) => issue.field),
    ["symbol", "limit"],
  );
});