import { describe, expect, it } from 'vitest';
import { createDownloads, formatEta, statusLine } from '$lib/turtlebox/tb-downloads.js';

describe('shared downloads state', () => {
  it('counts only running downloads for the badge and tracks speed', () => {
    let t = 0;
    const d = createDownloads({ now: () => t });
    d.update({ id: 'model:q', name: 'Qwen', total: 1e9, done: 0, state: 'running' });
    t = 1000;
    d.update({ id: 'model:q', done: 2e6 });
    expect(d.activeCount()).toBe(1);
    expect(statusLine(d.get('model:q')!)).toContain('2.00 MB/s');
    d.update({ id: 'model:q', state: 'paused' });
    expect(d.activeCount()).toBe(0);
    expect(d.groups().ongoing).toHaveLength(1);
  });
  it('writes time left in plain words', () => {
    expect(formatEta(15840)).toBe('4 hrs 24 mins left');
  });
});
