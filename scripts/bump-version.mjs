// Single source of truth for the app version is [workspace.package] in Cargo.toml.
//   node scripts/bump-version.mjs 1.2.3     set a new version everywhere
//   node scripts/bump-version.mjs --check   fail if any copy differs (used in CI)
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const cargoPath = join(root, 'Cargo.toml');
const jsonCopies = [
  'apps/desktop/src-tauri/tauri.conf.json',
  'apps/desktop/package.json',
  'apps/desktop/ui/package.json'
].map((p) => join(root, p));

const SEMVER = /^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/;
const VERSION_LINE = /^(version\s*=\s*)"([^"]+)"/m;

export function readCargoVersion(toml) {
  const section = toml.slice(toml.indexOf('[workspace.package]'));
  const m = VERSION_LINE.exec(section);
  if (!m) throw new Error('no version in [workspace.package]');
  return m[2];
}

export function setCargoVersion(toml, version) {
  const start = toml.indexOf('[workspace.package]');
  if (start < 0) throw new Error('no [workspace.package] section');
  return toml.slice(0, start) + toml.slice(start).replace(VERSION_LINE, `$1"${version}"`);
}

function main(argv) {
  const arg = argv[2];
  const toml = readFileSync(cargoPath, 'utf8');
  if (arg === '--check') {
    const want = readCargoVersion(toml);
    const bad = jsonCopies.filter((p) => JSON.parse(readFileSync(p, 'utf8')).version !== want);
    if (bad.length) {
      console.error(`Version drift (Cargo.toml says ${want}):\n` + bad.join('\n'));
      process.exit(1);
    }
    console.log(`version ${want} is consistent`);
    return;
  }
  if (!arg || !SEMVER.test(arg)) {
    console.error('usage: bump-version.mjs <x.y.z[-pre]> | --check');
    process.exit(2);
  }
  writeFileSync(cargoPath, setCargoVersion(toml, arg));
  for (const p of jsonCopies) {
    // Text replace of the first top-level "version" keeps each file's formatting.
    const text = readFileSync(p, 'utf8');
    const next = text.replace(/("version"\s*:\s*)"[^"]*"/, `$1"${arg}"`);
    if (next === text && JSON.parse(text).version !== arg) throw new Error(`no version field in ${p}`);
    writeFileSync(p, next);
  }
  console.log(`set version ${arg}; run \`cargo update --workspace\` to refresh Cargo.lock`);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main(process.argv);
