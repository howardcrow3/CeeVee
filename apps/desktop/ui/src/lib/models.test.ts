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
