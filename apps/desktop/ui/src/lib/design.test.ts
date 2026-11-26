import { describe, expect, it } from 'vitest';
import tokens from '../styles/turtlebox-tokens.css?raw';
import appCss from '../app.css?raw';

// The framework roles the components are written against.
const roles = ['paper', 'sidebar', 'surface', 'hover', 'selected', 'line', 'ink', 'body', 'muted', 'control', 'accent', 'accent-soft', 'on-ink', 'on-accent', 'danger', 'ok'];

describe('Turtlebox design tokens', () => {
  it('defines every role for light and dark', () => {
    const lightBlock = tokens.slice(tokens.indexOf(':root {'), tokens.indexOf('@media'));
    const darkBlock = tokens.slice(tokens.indexOf(':root[data-theme="dark"]'), tokens.indexOf('.tb-light'));
    for (const r of roles) {
      expect(lightBlock, `light ${r}`).toContain(`--${r}:`);
      expect(darkBlock, `dark ${r}`).toContain(`--${r}:`);
    }
  });

  it('follows the system theme and lets data-theme force one', () => {
    expect(tokens).toContain('prefers-color-scheme: dark');
    expect(tokens).toContain(':root:not([data-theme="light"])');
    expect(tokens).toContain(':root[data-theme="dark"]');
  });

  it('is sans-serif only', () => {
    expect(appCss).not.toMatch(/Cormorant/);
    expect(appCss).not.toMatch(/font-family:\s*['"]?(Georgia|Cormorant)/);
    expect(tokens).not.toMatch(/Cormorant|Georgia/);
  });

  it('keeps the names existing code uses', () => {
    for (const alias of ['--border-control', '--focus-ring', '--font-text', '--font-display', '--ease']) {
      expect(appCss).toContain(alias);
    }
  });
});
