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
