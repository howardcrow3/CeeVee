import { describe, expect, it } from 'vitest';
import { accepted, savedSummary } from './onboarding';

describe('onboarding', () => {
  it('keeps only the ticked changes in order', () => {
    const cs = [
      { change: 'a', text: 'A' },
      { change: 'b', text: 'B' },
      { change: 'c', text: 'C' }
    ];
    expect(accepted(cs, [true, false, true])).toEqual(['a', 'c']);
    expect(accepted(cs, [])).toEqual([]);
  });

  it('says what was saved', () => {
    expect(savedSummary(0)).toBe('Nothing was changed.');
    expect(savedSummary(1)).toBe('Saved 1 change to your profile.');
    expect(savedSummary(4)).toBe('Saved 4 changes to your profile.');
  });
});
