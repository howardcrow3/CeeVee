import { describe, expect, it } from 'vitest';
import { applyTheme, parseTheme } from './theme';

function fakeRoot() {
  const attrs = new Map<string, string>();
  return {
    attrs,
    setAttribute: (k: string, v: string) => void attrs.set(k, v),
    removeAttribute: (k: string) => void attrs.delete(k)
  };
}

describe('theme', () => {
  it('follows the system unless light or dark is stored', () => {
    expect(parseTheme(null)).toBe('system');
    expect(parseTheme('nonsense')).toBe('system');
    expect(parseTheme('dark')).toBe('dark');
    expect(parseTheme('light')).toBe('light');
  });
  it('forces a theme with data-theme and clears it for system', () => {
    const r = fakeRoot();
    applyTheme('dark', r);
    expect(r.attrs.get('data-theme')).toBe('dark');
    applyTheme('system', r);
    expect(r.attrs.has('data-theme')).toBe(false);
  });
});
