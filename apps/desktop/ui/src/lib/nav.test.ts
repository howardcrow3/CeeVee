import { describe, expect, it } from 'vitest';
import { isCurrent, sections } from './nav';

describe('nav', () => {
  it('marks only the home section current on /', () => {
    expect(sections.filter((s) => isCurrent(s.href, '/')).map((s) => s.label)).toEqual(['Chat']);
  });
  it('lists Job types before Jobs', () => {
    const labels = sections.map((x) => x.label);
    expect(labels.indexOf('Job types')).toBeLessThan(labels.indexOf('Jobs'));
  });
  it('keeps Settings out of the sidebar list', () => {
    expect(sections.map((x) => x.label)).not.toContain('Settings');
  });
  it('marks a section current for its own pages', () => {
    expect(isCurrent('/jobs', '/jobs')).toBe(true);
    expect(isCurrent('/jobs', '/jobs/42')).toBe(true);
    expect(isCurrent('/jobs', '/jobsite')).toBe(false);
  });
});
