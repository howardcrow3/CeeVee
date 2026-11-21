import { describe, expect, it } from 'vitest';
import { isCurrent, sections } from './nav';

describe('nav', () => {
  it('marks only the home section current on /', () => {
    expect(sections.filter((s) => isCurrent(s.href, '/')).map((s) => s.label)).toEqual(['Today']);
  });
  it('marks a section current for its own pages', () => {
    expect(isCurrent('/jobs', '/jobs')).toBe(true);
    expect(isCurrent('/jobs', '/jobs/42')).toBe(true);
    expect(isCurrent('/jobs', '/jobsite')).toBe(false);
  });
});
