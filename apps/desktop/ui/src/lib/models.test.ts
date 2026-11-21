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
