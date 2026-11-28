import { describe, expect, it } from 'vitest';
import { funnelText, goalText, importSummary, moveText, localToUtc, silenceText, tierLabel, workLine } from './people';

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

  it('turns a local time into UTC without milliseconds', () => {
    expect(localToUtc('2026-12-03T17:00')).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:00:00Z$/);
    expect(localToUtc('nonsense')).toBe('');
  });

  it('words the weekly goal', () => {
    expect(goalText(1, 3)).toBe('1 of 3 this week');
    expect(goalText(3, 3)).toBe('Goal met: 3 people this week.');
    expect(goalText(1, 1)).toBe('Goal met: 1 person this week.');
  });

  it('words the funnel and moves', () => {
    expect(funnelText(0, 0, 0)).toBe('');
    expect(funnelText(4, 2, 1)).toBe('Last 30 days: asked 4, 2 answered, 1 referred.');
    expect(moveText('Acme', 'Globex', 'Engineer')).toBe('Moved from Acme to Globex as Engineer');
    expect(moveText('', 'Globex', '')).toBe('Moved to Globex');
  });
});
