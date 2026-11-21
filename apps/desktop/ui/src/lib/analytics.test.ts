import { describe, expect, it } from 'vitest';
import { shouldAsk, statusLine, sentSummary } from './analytics';

const v = (consent: string, available = true) => ({ consent, available });

describe('analytics', () => {
  it('asks only when no choice has been made', () => {
    expect(shouldAsk(null)).toBe(false);
    expect(shouldAsk(v('unset'))).toBe(true);
    expect(shouldAsk(v('granted'))).toBe(false);
    expect(shouldAsk(v('denied'))).toBe(false);
  });
  it('says plainly what the state is', () => {
    expect(statusLine(v('unset'))).toContain('Nothing is sent');
    expect(statusLine(v('denied'))).toContain('Nothing is sent');
    expect(statusLine(v('granted'))).toContain('Sharing');
    expect(statusLine(v('granted', false))).toContain('nothing is sent');
  });
  it('promises no personal content', () => {
    expect(sentSummary).toContain('Never your resume');
  });
});
