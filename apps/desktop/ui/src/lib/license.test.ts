import { describe, expect, it } from 'vitest';
import { bannerText, canActivate, keyFromInput, statusText, type LicenseView } from './license';

const base: LicenseView = {
  state: 'trial', days_left: 10, ends_at: null, key_hint: null, seats: null, read_only: false, note: null
};

describe('license text', () => {
  it('stays quiet early in the trial and when licensed', () => {
    expect(bannerText(base)).toBeNull();
    expect(bannerText({ ...base, state: 'licensed' })).toBeNull();
  });
  it('warns in the last days', () => {
    expect(bannerText({ ...base, days_left: 3 })).toBe('Your trial ends in 3 days.');
    expect(bannerText({ ...base, days_left: 1 })).toBe('Your trial ends today or tomorrow.');
  });
  it('says read-only when expired', () => {
    expect(bannerText({ ...base, state: 'expired', days_left: null, read_only: true })).toMatch(/read-only/);
  });
  it('status line covers each state', () => {
    expect(statusText({ ...base, days_left: 1 })).toBe('Trial: 1 day left. Everything is unlocked.');
    expect(statusText({ ...base, days_left: 9 })).toBe('Trial: 9 days left. Everything is unlocked.');
    expect(statusText({ ...base, state: 'licensed', key_hint: 'CEEV-…-H3NZ8', seats: 2 })).toContain('CEEV-…-H3NZ8');
    expect(statusText({ ...base, state: 'expired' })).toMatch(/ended/);
  });
});

describe('key input', () => {
  it('needs something to send', () => {
    expect(canActivate('')).toBe(false);
    expect(canActivate('CEEV-')).toBe(false);
    expect(canActivate('CEEV-7KQ2M-9XB4D-TR5VW-H3NZ8')).toBe(true);
  });
  it('accepts a pasted activation link', () => {
    expect(keyFromInput('ceevee://activate?key=CEEV-7KQ2M-9XB4D-TR5VW-H3NZ8')).toBe('CEEV-7KQ2M-9XB4D-TR5VW-H3NZ8');
    expect(keyFromInput('  CEEV-7KQ2M  ')).toBe('CEEV-7KQ2M');
  });
});
