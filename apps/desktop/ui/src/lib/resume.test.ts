import { describe, expect, it } from 'vitest';
import { emptyJob, parseResume, serializeResume } from './resume';

const sample = JSON.stringify({
  basics: { name: 'Ada Lovelace', email: 'ada@example.com', location: { city: 'London', countryCode: 'GB' }, profiles: [{ network: 'x' }] },
  work: [{ name: 'Analytical Engines', position: 'Programmer', highlights: ['Wrote the first program'], url: 'https://e.example' }],
  skills: [{ name: 'Mathematics', level: 'expert' }],
  meta: { theme: 'plain' }
});

describe('resume form model', () => {
  it('reads the fields it knows', () => {
    const r = parseResume(sample)!;
    expect(r.name).toBe('Ada Lovelace');
    expect(r.location).toBe('London');
    expect(r.work[0].highlights).toEqual(['Wrote the first program']);
    expect(r.skills).toEqual(['Mathematics']);
  });
  it('returns null for text that is not JSON', () => {
    expect(parseResume('not json')).toBeNull();
  });
  it('keeps fields it does not know when writing back', () => {
    const r = parseResume(sample)!;
    r.name = 'Ada King';
    const out = JSON.parse(serializeResume(r, sample));
    expect(out.basics.name).toBe('Ada King');
    expect(out.basics.location.countryCode).toBe('GB');
    expect(out.basics.profiles).toHaveLength(1);
    expect(out.work[0].url).toBe('https://e.example');
    expect(out.meta.theme).toBe('plain');
  });
  it('drops blank bullets and blank skills, and adds new jobs', () => {
    const r = parseResume(sample)!;
    r.work[0].highlights.push('  ');
    r.skills.push('');
    r.work.push({ ...emptyJob(), name: 'New Co' });
    const out = JSON.parse(serializeResume(r, sample));
    expect(out.work[0].highlights).toHaveLength(1);
    expect(out.skills).toHaveLength(1);
    expect(out.work[1].name).toBe('New Co');
  });
  it('works from an empty starting point', () => {
    const r = parseResume('{}')!;
    expect(r.work).toEqual([]);
    expect(JSON.parse(serializeResume(r, '{}')).basics.name).toBe('');
  });
});
