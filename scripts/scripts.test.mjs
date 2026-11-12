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
