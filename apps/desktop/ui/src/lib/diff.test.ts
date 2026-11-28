import { describe, expect, it } from 'vitest';
import { diffLines, diffSummary, isFlagged } from './diff';

describe('diffLines', () => {
  it('marks what was added and what was replaced', () => {
    const d = diffLines('Ada\nBuilt dashboards\nLed a team', 'Ada\nBuilt SQL dashboards\nLed a team');
    expect(d).toEqual([
      { kind: 'same', text: 'Ada' },
      { kind: 'del', text: 'Built dashboards' },
      { kind: 'add', text: 'Built SQL dashboards' },
      { kind: 'same', text: 'Led a team' }
    ]);
  });
  it('ignores blank lines and handles identical text', () => {
    const d = diffLines('a\n\nb', 'a\nb\n');
    expect(d.every((l) => l.kind === 'same')).toBe(true);
    expect(diffSummary(d)).toBe('No changes from your base resume.');
  });
  it('summarises counts', () => {
    expect(diffSummary(diffLines('a\nb', 'a\nc\nd'))).toBe('2 lines added or reworded, 1 line replaced.');
  });
});

describe('isFlagged', () => {
  it('finds a flagged phrase in a line, ignoring case', () => {
    expect(isFlagged('Managed 40 people', ['40 people'])).toBe(true);
    expect(isFlagged('Managed a team', ['40 people', ''])).toBe(false);
  });
});
