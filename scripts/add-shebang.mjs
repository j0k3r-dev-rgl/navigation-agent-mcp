#!/usr/bin/env node
import { readFileSync, writeFileSync, chmodSync } from 'node:fs';

const filePath = process.argv[2];
if (!filePath) {
  console.error('Usage: add-shebang.mjs <file>');
  process.exit(1);
}

const content = readFileSync(filePath, 'utf8');
writeFileSync(filePath, '#!/usr/bin/env node\n' + content);
chmodSync(filePath, 0o755);
console.log(`Added shebang and set executable: ${filePath}`);
