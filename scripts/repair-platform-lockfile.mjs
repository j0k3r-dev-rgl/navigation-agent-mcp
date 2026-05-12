#!/usr/bin/env node
/**
 * Repairs package-lock entries for platform-specific optional packages.
 *
 * npm 11 can leave workspace-local optional dependency placeholders without a
 * version when the next release version is not published yet. A subsequent
 * `npm ci` then fails with `Invalid Version:`. This script writes the minimal
 * package metadata npm needs to consume the lockfile on the next release run.
 *
 * Usage: node scripts/repair-platform-lockfile.mjs [version]
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const root = resolve(new URL('..', import.meta.url).pathname);
const lockfilePath = resolve(root, 'package-lock.json');

const explicitVersion = process.argv[2];
if (explicitVersion && !/^\d+\.\d+\.\d+([-.][0-9A-Za-z.-]+)?$/.test(explicitVersion)) {
  console.error('Usage: repair-platform-lockfile.mjs [semver]');
  process.exit(1);
}

const platformPackages = [
  'mcp-server-linux-x64',
  'mcp-server-linux-arm64',
  'mcp-server-darwin-x64',
  'mcp-server-darwin-arm64',
  'mcp-server-win32-x64',
];

const lockfile = JSON.parse(readFileSync(lockfilePath, 'utf8'));
lockfile.packages ??= {};

for (const packageDir of platformPackages) {
  const manifestPath = resolve(root, 'packages', packageDir, 'package.json');
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
  const version = explicitVersion ?? manifest.version;
  const entryPath = `packages/mcp-server/node_modules/${manifest.name}`;
  const encodedName = manifest.name.replace('/', '%2f');

  lockfile.packages[entryPath] = {
    version,
    resolved: `https://registry.npmjs.org/${encodedName}/-/${packageDir}-${version}.tgz`,
    cpu: manifest.cpu,
    license: manifest.license,
    optional: true,
    os: manifest.os,
  };
}

writeFileSync(lockfilePath, JSON.stringify(lockfile, null, 2) + '\n');
console.log('Repaired platform optional dependency entries in package-lock.json');
