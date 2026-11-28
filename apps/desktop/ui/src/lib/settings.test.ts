import { describe, expect, it } from 'vitest';
import { settingsTabs, tabOrDefault } from './settings';

describe('settings tabs', () => {
  it('has unique ids', () => {
    expect(new Set(settingsTabs.map((t) => t.id)).size).toBe(settingsTabs.length);
  });
  it('falls back to the first tab', () => {
    expect(tabOrDefault('accounts')).toBe('accounts');
    expect(tabOrDefault('nope')).toBe('general');
    expect(tabOrDefault(undefined)).toBe('general');
  });
});
