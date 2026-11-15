import { describe, expect, it } from 'vitest';
import { cleanBoards, searchSummary } from './boards';

describe('cleanBoards', () => {
  it('drops blank rows and trims', () => {
    expect(
      cleanBoards([
        { kind: 'lever', token: ' acme ', company: ' Acme ' },
        { kind: 'lever', token: '', company: '  ' }
      ])
    ).toEqual([{ kind: 'lever', token: 'acme', company: 'Acme' }]);
  });
});

describe('searchSummary', () => {
  it('uses plain counts', () => {
    expect(searchSummary({ boards_ok: 1, new: 1, failures: [] })).toBe(
      'Found 1 new job at 1 company.'
    );
    expect(searchSummary({ boards_ok: 3, new: 0, failures: ['x'] })).toBe(
      'Found 0 new jobs at 3 companies. 1 could not be read.'
    );
  });
});

describe('starter companies', () => {
  it('are all valid board names', async () => {
    const { starterBoards } = await import('./boards');
    for (const b of starterBoards) expect(b.token).toMatch(/^[A-Za-z0-9_-]+$/);
    expect(new Set(starterBoards.map((b) => `${b.kind}:${b.token}`)).size).toBe(starterBoards.length);
  });
  it('are added without duplicating what is there', async () => {
    const { withStarters, starterBoards } = await import('./boards');
    const once = withStarters([{ kind: 'greenhouse', token: 'Stripe', company: 'Stripe Inc' }]);
    expect(once).toHaveLength(starterBoards.length);
    expect(once[0].company).toBe('Stripe Inc');
    expect(withStarters(once)).toHaveLength(once.length);
  });
});
