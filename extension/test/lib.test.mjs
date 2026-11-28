import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

const L = createRequire(import.meta.url)('../lib.js');

test('labels are tidied', () => {
  assert.equal(L.cleanLabel('  First   name *  '), 'First name');
  assert.equal(L.cleanLabel('Email (required)'), 'Email');
  assert.equal(L.cleanLabel(null), '');
});

test('the best label wins', () => {
  assert.equal(L.pickLabel({ labelText: 'Phone', placeholder: 'x' }), 'Phone');
  assert.equal(L.pickLabel({ labelText: ' ', ariaLabel: 'Your name', name: 'n' }), 'Your name');
  assert.equal(L.pickLabel({ name: 'first_name' }), 'first_name');
  assert.equal(L.pickLabel({}), '');
});

test('only ordinary controls are read', () => {
  assert.equal(L.controlKind('INPUT', 'email'), 'text');
  assert.equal(L.controlKind('input', undefined), 'text');
  assert.equal(L.controlKind('input', 'file'), 'file');
  assert.equal(L.controlKind('input', 'checkbox'), 'checkbox');
  assert.equal(L.controlKind('textarea', null), 'textarea');
  assert.equal(L.controlKind('select', null), 'select');
  for (const t of ['hidden', 'submit', 'button', 'radio', 'password', 'image']) {
    assert.equal(L.controlKind('input', t), null, t);
  }
  assert.equal(L.controlKind('div', null), null);
});

test('the summary line counts what matters', () => {
  const list = [
    { state: 'needs_you' },
    { state: 'check_this' },
    { state: 'left_blank' },
    { state: 'filled' },
    { state: 'filled' }
  ];
  assert.equal(L.summaryText(list, false), '3 filled, 1 needs you, 1 to check, 1 left blank.');
  assert.equal(L.summaryText([{ state: 'filled' }], true), '1 filled. Ready for you to review and submit.');
  assert.equal(L.summaryText([], false), 'Nothing to fill on this page.');
});

test('the app is looked for on ten ports', () => {
  const p = L.candidatePorts();
  assert.equal(p.length, 10);
  assert.equal(p[0], 47831);
  assert.equal(p[9], 47840);
});

test('the background script looks on the same ports as the app and the helpers', async () => {
  const { readFile } = await import('node:fs/promises');
  const src = await readFile(new URL('../background.js', import.meta.url), 'utf8');
  assert.match(src, /const FIRST_PORT = 47831;/);
  assert.match(src, /const PORT_COUNT = 10;/);
  const rust = await readFile(new URL('../../crates/ceevee-bridge/src/server.rs', import.meta.url), 'utf8');
  assert.match(rust, /pub const FIRST_PORT: u16 = 47831;/);
  assert.match(rust, /pub const PORT_COUNT: u16 = 10;/);
});

test('the manifest asks only for the job sites and this computer', async () => {
  const { readFile } = await import('node:fs/promises');
  const m = JSON.parse(await readFile(new URL('../manifest.json', import.meta.url), 'utf8'));
  assert.deepEqual(m.permissions, ['storage']);
  assert.deepEqual(m.host_permissions, ['http://127.0.0.1/*', 'http://localhost/*']);
  for (const pattern of m.content_scripts[0].matches) assert.match(pattern, /^https:\/\//);
});
