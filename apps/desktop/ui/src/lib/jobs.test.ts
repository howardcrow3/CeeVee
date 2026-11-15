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

describe('prepareSummary', () => {
  it('lists what is left for the person', async () => {
    const { prepareSummary } = await import('./jobs');
    const t = prepareSummary({ filled: 5, missing: ['Visa sponsorship'], resume: 'Base resume' });
    expect(t).toContain('Filled 5 fields');
    expect(t).toContain('Visa sponsorship');
    expect(t).toContain('press Submit yourself');
  });
  it('says when there was no resume', async () => {
    const { prepareSummary } = await import('./jobs');
    expect(prepareSummary({ filled: 1, missing: [], resume: null })).toContain('No resume was attached');
  });
});
