import { describe, expect, it } from 'vitest';
import { matchText, placeText, statusWord } from './jobs';

describe('jobs helpers', () => {
  it('words the status', () => {
    expect(statusWord('saved')).toBe('Saved');
    expect(statusWord('new')).toBe('New');
  });
  it('formats the match', () => {
    expect(matchText(0.924)).toBe('92%');
    expect(matchText(null)).toBe('Not scored');
  });
  it('handles a missing location', () => {
    expect(placeText({ location: null })).toBe('Location not listed');
  });
});
