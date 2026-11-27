import { describe, expect, it } from 'vitest';
import { describeCategory, newCategory, splitList } from './categories';

describe('categories', () => {
  it('splits lists on commas and new lines', () => {
    expect(splitList('a, b ,, c\nd')).toEqual(['a', 'b', 'c', 'd']);
    expect(splitList('  ')).toEqual([]);
  });

  it('builds a category from the form', () => {
    const c = newCategory({
      name: ' PM ',
      titles: 'product manager, program manager',
      keywords: '',
      locations: 'Columbus',
      avoid: 'intern',
      remote_ok: true
    });
    expect(c.name).toBe('PM');
    expect(c.titles).toHaveLength(2);
    expect(c.active).toBe(true);
  });

  it('describes what it looks for', () => {
    const c = newCategory({ name: 'x', titles: 'engineer', keywords: '', locations: '', avoid: '', remote_ok: true });
    expect(describeCategory(c)).toBe('titles: engineer; places: remote');
    expect(describeCategory(newCategory({ name: 'x', titles: '', keywords: '', locations: '', avoid: '', remote_ok: false }))).toBe('No rules yet');
  });
});

import { edited, formFrom } from './categories';

describe('editing', () => {
  const saved = { id: 7, name: 'PM', titles: ['product manager', 'pm'], keywords: [], locations: ['Columbus'], remote_ok: false, avoid: ['intern'], active: false };
  it('fills the form from a saved job type', () => {
    expect(formFrom(saved)).toEqual({ name: 'PM', titles: 'product manager, pm', keywords: '', locations: 'Columbus', avoid: 'intern', remote_ok: false });
  });
  it('keeps the id and on/off state after an edit', () => {
    const f = { ...formFrom(saved), titles: 'designer', name: ' Design ' };
    const e = edited(saved, f);
    expect(e).toMatchObject({ id: 7, active: false, name: 'Design', titles: ['designer'] });
  });
});
