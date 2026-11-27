import { describe, expect, it } from 'vitest';
import { importSummary, silenceText, tierLabel, workLine } from './people';

describe('people', () => {
  it('labels tiers and falls back to the raw value', () => {
    expect(tierLabel('close')).toBe('Close');
    expect(tierLabel('odd')).toBe('odd');
  });

  it('joins title and company', () => {
    expect(workLine('Engineer', 'Acme')).toBe('Engineer at Acme');
    expect(workLine('', 'Acme')).toBe('Acme');
    expect(workLine('', '')).toBe('');
  });

  it('summarizes an import', () => {
    expect(importSummary({ added: 1, matched: 0, skipped: 0 })).toBe('Added 1 person.');
    expect(importSummary({ added: 3, matched: 2, skipped: 1 })).toBe(
      'Added 3 people, filled in details for 2 you already had, skipped 1 with no name.'
    );
  });

  it('words how long it has been', () => {
    expect(silenceText(5, true)).toBe('Not in touch since you added them');
    expect(silenceText(45, false)).toBe('Last in touch 45 days ago');
    expect(silenceText(95, false)).toBe('Last in touch about 3 months ago');
    expect(silenceText(400, false)).toBe('Last in touch over a year ago');
  });
});
