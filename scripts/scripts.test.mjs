import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readCargoVersion, setCargoVersion } from './bump-version.mjs';
import { buildManifest } from './make-manifest.mjs';

const toml = '[workspace]\nmembers = []\n\n[workspace.package]\nversion = "0.1.0"\nedition = "2021"\n\n[workspace.dependencies]\nsqlx = { version = "0.8" }\n';

test('reads and rewrites only the workspace package version', () => {
  assert.equal(readCargoVersion(toml), '0.1.0');
  const out = setCargoVersion(toml, '1.2.3-beta.1');
  assert.equal(readCargoVersion(out), '1.2.3-beta.1');
  assert.match(out, /sqlx = \{ version = "0.8" \}/);
});

test('manifest lists signed platforms with rollout', () => {
  const m = buildManifest({
    version: '1.0.0', base: 'https://dl.example/releases', percent: 10, pubDate: '2027-01-01T00:00:00Z',
    files: {
      'windows-x86_64': { file: 'CeeVee_1.0.0_x64-setup.exe', signature: 'SIG\n' },
      'darwin-aarch64': { file: 'CeeVee.app.tar.gz', signature: undefined }
    }
  });
  assert.deepEqual(Object.keys(m.platforms), ['windows-x86_64']);
  assert.equal(m.platforms['windows-x86_64'].url, 'https://dl.example/releases/1.0.0/CeeVee_1.0.0_x64-setup.exe');
  assert.equal(m.platforms['windows-x86_64'].signature, 'SIG');
  assert.deepEqual(m.rollout, { percent: 10, paused: false });
});

test('manifest without any signed file is an error', () => {
  assert.throws(() => buildManifest({ version: '1.0.0', base: 'x', files: { a: { file: 'f', signature: undefined } } }));
});

import { summarize, toMarkdown, toHtml, verdict, threshold } from './eval-report.mjs';

const rep = (model, task, mean, cases) => ({ model, task, version: 1, mean_score: mean, failed: 0, skipped: 0, total: cases.length, seconds: 1, cases: cases.map(([name, score]) => ({ name, score })) });

test('eval report compares models per task and applies thresholds without failing', () => {
  const results = [
    rep('small', 'extract_posting', 0.4, [['a', 0.4], ['b', null]]),
    rep('big', 'extract_posting', 0.9, [['a', 1], ['b', 0.8]]),
  ];
  const th = { default: { extract_posting: null }, models: { big: { extract_posting: 0.8 } } };
  const sum = summarize(results, th);
  assert.deepEqual(sum.models, ['big', 'small']);
  assert.equal(sum.cells['big|extract_posting'].verdict, 'pass');
  assert.equal(sum.cells['small|extract_posting'].verdict, 'not set');
  const md = toMarkdown(sum);
  assert.match(md, /\| big \| 0\.90 \(pass, floor 0\.80\)/);
  assert.match(md, /\| b \| 0\.80 \| FAIL \|/);
  assert.match(toHtml(sum), /<table>/);
});

test('thresholds fall back to the default and a missing floor is "not set"', () => {
  assert.equal(threshold({ default: { t: 0.5 }, models: {} }, 'm', 't'), 0.5);
  assert.equal(threshold({}, 'm', 't'), null);
  assert.equal(verdict(0.2, 0.5), 'fail');
  assert.equal(verdict(0.2, null), 'not set');
});

import { readFileSync } from 'node:fs';

test('eval models have pinned hashes and match the model catalog', () => {
  const models = JSON.parse(readFileSync(new URL('../evals/models.json', import.meta.url), 'utf8')).models;
  const catalog = JSON.parse(readFileSync(new URL('../crates/ceevee-ai/catalog.json', import.meta.url), 'utf8')).models;
  assert.ok(models.length >= 3);
  const ids = new Set();
  for (const m of models) {
    assert.ok(!ids.has(m.id), `duplicate ${m.id}`);
    ids.add(m.id);
    assert.match(m.sha256, /^[0-9a-f]{64}$/, m.id);
    assert.match(m.url, /^https:\/\/huggingface\.co\//, m.id);
    assert.ok(Number.isInteger(m.budget_secs) && m.budget_secs > 0, m.id);
    assert.equal(typeof m.cache, 'boolean', m.id);
    // A model that ships in the catalog must be scored with the very same file.
    const shipped = catalog.find((c) => c.url === m.url);
    if (shipped) assert.equal(shipped.sha256, m.sha256, m.id);
  }
  for (const c of catalog.filter((x) => x.role === 'general')) {
    assert.ok(models.some((m) => m.url === c.url), `${c.id} is shipped but not scored`);
  }
});

import { buildRun, mergeHistory, buildData, render } from './eval-dashboard.mjs';

test('dashboard keeps one entry per run and embeds data safely', () => {
  const r = [{ model: 'm', task: 'extract_posting', version: 1, mean_score: 0.5, failed: 1, skipped: 2, total: 5, seconds: 3,
    cases: [{ name: 'a</script>', score: 1, seconds: 1, attempts: 1, error: null, fields: { x: 1 } }, { name: 'b', score: 0, error: 'bad' }] }];
  const run = buildRun(r, { id: 'run-1', commit: 'abc', date: '2026-01-01' });
  assert.equal(run.results[0].cases[1].score, null);
  let h = mergeHistory({ runs: [] }, run);
  h = mergeHistory(h, { ...run, commit: 'def' });
  assert.equal(h.runs.length, 1);
  assert.equal(h.runs[0].commit, 'def');
  const tpl = readFileSync(new URL('./eval-dashboard.template.html', import.meta.url), 'utf8');
  const html = render(tpl, buildData({ history: h, models: [{ id: 'm' }], thresholds: {} }));
  assert.ok(!html.includes('/*__DATA__*/'));
  assert.ok(!html.includes('a</script>'));
});

test('committed history matches the models and tasks the dashboard expects', () => {
  const h = JSON.parse(readFileSync(new URL('../evals/history.json', import.meta.url), 'utf8'));
  const ids = JSON.parse(readFileSync(new URL('../evals/models.json', import.meta.url), 'utf8')).models.map((m) => m.id);
  for (const run of h.runs) for (const x of run.results) assert.ok(ids.includes(x.model), x.model);
});

import { render as renderMd, page } from './customer-pages.mjs';

test('customer pages: headings, lists, tables and escaping', () => {
  const html = renderMd('# Title\n\nA **bold** <b>\n\n- one\n- two\n\n| a | b |\n|---|---|\n| 1 | `x` |\n');
  assert.match(html, /<h1>Title<\/h1>/);
  assert.match(html, /<strong>bold<\/strong> &lt;b&gt;/);
  assert.match(html, /<ul><li>one<\/li><li>two<\/li><\/ul>/);
  assert.match(html, /<td><code>x<\/code><\/td>/);
  assert.match(page('T', html), /^<!doctype html>/);
});

// Guard for the macOS "damaged, move to Trash" bug: an app built with --no-sign has no sealed
// resources, and Apple silicon refuses it once it is quarantined. Ad-hoc sign instead.
test('no workflow builds the mac app with --no-sign, and the test build verifies it', async () => {
  const { readdirSync, readFileSync } = await import('node:fs');
  const dir = new URL('../.github/workflows/', import.meta.url);
  for (const f of readdirSync(dir).filter((n) => n.endsWith('.yml'))) {
    const text = readFileSync(new URL(f, dir), 'utf8');
    const code = text.split('\n').filter((l) => !l.trim().startsWith('#')).join('\n');
    assert.ok(!code.includes('--no-sign'), `${f} uses --no-sign`);
  }
  const tb = readFileSync(new URL('test-build.yml', dir), 'utf8');
  // The Test build must be Developer ID signed and notarized, and prove it, so it installs normally.
  assert.match(tb, /environment: release-build/);
  assert.match(tb, /APPLE_CERTIFICATE: /);
  assert.match(tb, /verify-mac-dmg\.sh .*--require-notarized/);
  assert.ok(!/APPLE_SIGNING_IDENTITY: '-'/.test(tb), 'test build must not be ad-hoc signed');
});

// Keychain items must live under CeeVee's own service name (the bundle identifier), so the macOS
// prompt names CeeVee and each item is easy to find and delete in Keychain Access.
test('keychain service names equal the bundle identifier', async () => {
  const { readFileSync } = await import('node:fs');
  const root = new URL('../', import.meta.url);
  const id = JSON.parse(readFileSync(new URL('apps/desktop/src-tauri/tauri.conf.json', root), 'utf8')).identifier;
  assert.equal(id, 'studio.turtlebox.ceevee');
  assert.match(readFileSync(new URL('crates/ceevee-license/src/anchor.rs', root), 'utf8'), new RegExp(`Entry::new\\("${id}"`));
  assert.match(readFileSync(new URL('crates/ceevee-mail/src/secrets.rs', root), 'utf8'), new RegExp(`SERVICE: &str = "${id}"`));
});
