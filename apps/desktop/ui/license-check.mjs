// Fails if any production npm dependency has a license outside the allowlist.
import { execFileSync } from 'node:child_process';

const ALLOWED = new Set(['MIT', 'Apache-2.0', 'BSD-2-Clause', 'BSD-3-Clause', 'ISC', 'Zlib', '0BSD', 'MPL-2.0', 'CC0-1.0', 'Unlicense', 'BlueOak-1.0.0', 'Python-2.0']);
const out = JSON.parse(execFileSync('pnpm', ['licenses', 'list', '--prod', '--json'], { encoding: 'utf8', shell: process.platform === 'win32' }));

const bad = [];
for (const [license, pkgs] of Object.entries(out)) {
  const parts = license.replace(/[()]/g, '').split(/\s+(?:OR|AND)\s+/);
  // An OR expression passes if any alternative is allowed; AND needs all.
  const isAnd = /\sAND\s/.test(license);
  const ok = isAnd ? parts.every((p) => ALLOWED.has(p)) : parts.some((p) => ALLOWED.has(p));
  if (!ok) bad.push(`${license}: ${pkgs.map((p) => p.name).join(', ')}`);
}
if (bad.length) {
  console.error('Disallowed licenses:\n' + bad.join('\n'));
  process.exit(1);
}
console.log(`npm licenses OK (${Object.keys(out).join(', ') || 'none'})`);
