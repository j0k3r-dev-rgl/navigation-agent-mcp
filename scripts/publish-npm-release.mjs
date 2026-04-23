#!/usr/bin/env node
import { execFileSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { resolve } from 'node:path';

const version = process.argv[2];

if (!version || !/^\d+\.\d+\.\d+([-.][0-9A-Za-z.-]+)?$/.test(version)) {
  console.error('Usage: publish-npm-release.mjs <semver>');
  process.exit(1);
}

const root = resolve(new URL('..', import.meta.url).pathname);

const binaryPackages = [
  {
    name: '@navigation-agent/mcp-server-linux-x64',
    dir: 'packages/mcp-server-linux-x64',
    binary: 'navigation-engine',
  },
  {
    name: '@navigation-agent/mcp-server-linux-arm64',
    dir: 'packages/mcp-server-linux-arm64',
    binary: 'navigation-engine',
  },
  {
    name: '@navigation-agent/mcp-server-darwin-x64',
    dir: 'packages/mcp-server-darwin-x64',
    binary: 'navigation-engine',
  },
  {
    name: '@navigation-agent/mcp-server-darwin-arm64',
    dir: 'packages/mcp-server-darwin-arm64',
    binary: 'navigation-engine',
  },
  {
    name: '@navigation-agent/mcp-server-win32-x64',
    dir: 'packages/mcp-server-win32-x64',
    binary: 'navigation-engine.exe',
  },
];

for (const pkg of binaryPackages) {
  const binaryPath = resolve(root, pkg.dir, pkg.binary);
  if (!existsSync(binaryPath)) {
    console.error(`Missing binary for ${pkg.name}: ${binaryPath}`);
    process.exit(1);
  }
}

const run = (args) => {
  execFileSync('npm', args, {
    cwd: root,
    stdio: 'inherit',
    env: process.env,
  });
};

console.log(`Publishing npm release ${version}`);

for (const pkg of binaryPackages) {
  console.log(`\n→ Publishing ${pkg.name}`);
  run(['publish', `./${pkg.dir}`, '--access', 'public']);
}

console.log('\n→ Publishing @navigation-agent/mcp-server');
run(['publish', '--workspace', '@navigation-agent/mcp-server', '--access', 'public']);

console.log(`\nPublished npm release ${version}`);
