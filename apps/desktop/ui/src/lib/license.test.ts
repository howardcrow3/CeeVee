import { describe, expect, it } from 'vitest';
import { canActivate, keyFromInput, menuLicense, pillText, statusText, type LicenseView } from './license';

const base: LicenseView = {
  state: 'trial', days_left: 10, ends_at: null, key_hint: null, seats: null, read_only: false, note: null
};

describe('license text', () => {
  it('pill shows days left during the trial and Licensed once a key is saved', () => {
    expect(pillText(null)).toBeNull();
    expect(pillText(base)).toEqual({ text: 'Trial · 10 days left', tone: 'ok' });
    expect(pillText({ ...base, days_left: 1 })).toEqual({ text: 'Trial · 1 day left', tone: 'soon' });
    expect(pillText({ ...base, state: 'licensed' })).toEqual({ text: 'Licensed', tone: 'ok' });
  });
  it('pill says read-only when expired', () => {
    expect(pillText({ ...base, state: 'expired', days_left: null, read_only: true })?.tone).toBe('ended');
    expect(pillText({ ...base, state: 'expired', days_left: null })?.text).toMatch(/read-only/);
  });
  it('menu offers Buy during and after the trial, not once licensed', () => {
    expect(menuLicense(null)).toBeNull();
    expect(menuLicense(base)?.buy).toBe(true);
    expect(menuLicense({ ...base, state: 'expired', days_left: null })).toMatchObject({ buy: true, tone: 'ended' });
    expect(menuLicense({ ...base, state: 'licensed', seats: 2 })).toMatchObject({ buy: false, title: 'Licensed' });
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
