// Builds an updater channel manifest (TECHNICAL_PLAN 17.5) from a release folder.
//   node scripts/make-manifest.mjs --version 1.0.0 --dir ./dist --notes notes.md \
//        --base https://dl.turtlebox.studio/ceevee/releases --percent 10 > stable.json
// Expects <dir>/CeeVee_<v>_x64-setup.exe(.sig) and <dir>/CeeVee.app.tar.gz(.sig) when present.
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

export function buildManifest({ version, notes = '', base, percent = 100, paused = false, minVersion, files, pubDate }) {
  const platforms = {};
  for (const [platform, name] of Object.entries(files)) {
    if (name.signature === undefined) continue; // not built for this release
    platforms[platform] = { url: `${base}/${version}/${name.file}`, signature: name.signature.trim() };
  }
  if (!Object.keys(platforms).length) throw new Error('no signed platform files found');
  const manifest = {
    version,
    notes,
    pub_date: pubDate ?? new Date().toISOString(),
    platforms,
    rollout: { percent, paused }
  };
  if (minVersion) manifest.min_version = minVersion;
  return manifest;
}

function readSigned(dir, file) {
  const p = join(dir, file);
  return { file, signature: existsSync(`${p}.sig`) ? readFileSync(`${p}.sig`, 'utf8') : undefined };
}

function main(argv) {
  const opt = {};
  for (let i = 2; i < argv.length; i += 2) opt[argv[i].replace(/^--/, '')] = argv[i + 1];
  if (!opt.version || !opt.dir || !opt.base) {
    console.error('usage: make-manifest.mjs --version V --dir DIR --base URL [--notes FILE] [--percent N] [--paused true] [--min-version V]');
    process.exit(2);
  }
  const v = opt.version;
  const manifest = buildManifest({
    version: v,
    notes: opt.notes ? readFileSync(opt.notes, 'utf8').trim() : '',
    base: opt.base.replace(/\/$/, ''),
    percent: Number(opt.percent ?? 100),
    paused: opt.paused === 'true',
    minVersion: opt['min-version'],
    files: {
      'windows-x86_64': readSigned(opt.dir, `CeeVee_${v}_x64-setup.exe`),
      'darwin-aarch64': readSigned(opt.dir, 'CeeVee.app.tar.gz')
    }
  });
  process.stdout.write(JSON.stringify(manifest, null, 2) + '\n');
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main(process.argv);
