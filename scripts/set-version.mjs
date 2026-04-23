#!/usr/bin/env node
/**
 * Updates version across release-managed package manifests.
 * Also syncs optionalDependencies, package-lock, and Cargo.toml.
 *
 * Usage: node scripts/set-version.mjs <version>
 * Example: node scripts/set-version.mjs 1.2.3
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const version = process.argv[2];
if (!version || !/^\d+\.\d+\.\d+/.test(version)) {
  console.error('Usage: set-version.mjs <semver>  (e.g. 1.2.3)');
  process.exit(1);
}

const root = new URL('..', import.meta.url).pathname;

const packagePaths = [
  'package.json',
  'packages/contract-tests/package.json',
  'packages/mcp-server/package.json',
  'packages/mcp-server-linux-x64/package.json',
  'packages/mcp-server-linux-arm64/package.json',
  'packages/mcp-server-darwin-x64/package.json',
  'packages/mcp-server-darwin-arm64/package.json',
  'packages/mcp-server-win32-x64/package.json',
];

for (const relPath of packagePaths) {
  const absPath = resolve(root, relPath);
  const pkg = JSON.parse(readFileSync(absPath, 'utf8'));

  pkg.version = version;

  // Sync optionalDependencies that belong to this monorepo
  if (pkg.optionalDependencies) {
    for (const key of Object.keys(pkg.optionalDependencies)) {
      if (key.startsWith('@navigation-agent/')) {
        pkg.optionalDependencies[key] = version;
      }
    }
  }

  writeFileSync(absPath, JSON.stringify(pkg, null, 2) + '\n');
  console.log(`  ${relPath} → ${version}`);
}

console.log(`\nAll packages set to ${version}`);

const lockfilePath = resolve(root, 'package-lock.json');
const lockfile = JSON.parse(readFileSync(lockfilePath, 'utf8'));

lockfile.version = version;

if (lockfile.packages?.['']) {
  lockfile.packages[''].version = version;
}

for (const relPath of packagePaths.filter((path) => path !== 'package.json')) {
  const packageEntry = lockfile.packages?.[relPath];
  if (packageEntry) {
    packageEntry.version = version;
  }
}

writeFileSync(lockfilePath, JSON.stringify(lockfile, null, 2) + '\n');
console.log(`  package-lock.json → ${version}`);

const cargoTomlPath = resolve(root, 'crates/navigation-engine/Cargo.toml');
const cargoToml = readFileSync(cargoTomlPath, 'utf8').replace(
  /^version = ".*"$/m,
  `version = "${version}"`,
);
writeFileSync(cargoTomlPath, cargoToml);
console.log(`  crates/navigation-engine/Cargo.toml → ${version}`);
