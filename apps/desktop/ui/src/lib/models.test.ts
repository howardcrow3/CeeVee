import { describe, expect, it } from 'vitest';
import { percent, sizeText } from './models';

describe('models', () => {
  it('computes percent safely', () => {
    expect(percent(0, 0)).toBe(0);
    expect(percent(500, 2000)).toBe(25);
    expect(percent(3000, 2000)).toBe(100);
  });
  it('writes sizes in plain words', () => {
    expect(sizeText(37)).toBe('37 MB');
    expect(sizeText(2497)).toBe('2.5 GB');
  });
});

import { setupView } from './models';

describe('setupView', () => {
  it('is ready once the model is installed', () => {
    expect(setupView({ installed: true }, null, '')).toBe('ready');
  });
  it('offers the download on a fresh install', () => {
    expect(setupView({ installed: false }, null, '')).toBe('offer');
    expect(setupView({ installed: false }, { running: false, error: '' }, '')).toBe('offer');
  });
  it('shows progress while downloading, even before status loads', () => {
    expect(setupView({ installed: false }, { running: true, error: '' }, '')).toBe('downloading');
    expect(setupView(null, { running: true, error: '' }, '')).toBe('downloading');
  });
  it('reports a failure instead of staying silent', () => {
    expect(setupView({ installed: false }, { running: false, error: 'offline' }, '')).toBe('failed');
    expect(setupView(null, null, 'no memory')).toBe('failed');
  });
  it('is checking until the status arrives', () => {
    expect(setupView(null, null, '')).toBe('checking');
  });
});

import { modelItem } from './models';

describe('modelItem', () => {
  const m = { id: 'q', name: 'Qwen', installed: false };
  const p = (o = {}) => ({ running: false, downloaded_mb: 0, total_mb: 0, error: '', ...o });
  it('shows nothing before a download', () => expect(modelItem(m, p(), 'none', false)).toBeNull());
  it('runs in bytes', () => {
    const i = modelItem(m, p({ running: true, downloaded_mb: 100, total_mb: 400 }), 'none', true);
    expect(i).toMatchObject({ state: 'running', done: 100e6, total: 400e6, id: 'model:q' });
  });
  it('pause and cancel follow the user intent', () => {
    expect(modelItem(m, p({ downloaded_mb: 5, total_mb: 9 }), 'pause', true)?.state).toBe('paused');
    expect(modelItem(m, p({ downloaded_mb: 5, total_mb: 9 }), 'cancel', true)?.state).toBe('canceled');
  });
  it('reports a failure with the reason', () => {
    expect(modelItem(m, p({ error: 'disk full' }), 'none', true)).toMatchObject({ state: 'failed', error: 'disk full' });
  });
  it('lists done only after a download this session', () => {
    const done = { ...m, installed: true };
    expect(modelItem(done, p({ total_mb: 9, downloaded_mb: 9 }), 'none', false)).toBeNull();
    expect(modelItem(done, p({ total_mb: 9, downloaded_mb: 9 }), 'none', true)?.state).toBe('done');
  });
});
