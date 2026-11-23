import { describe, expect, it } from 'vitest';
import { byMatch, draftLabel, draftNote, knownText, matchText, placeText, statusWord } from './jobs';

describe('jobs helpers', () => {
  it('words the status', () => {
    expect(statusWord('saved')).toBe('Saved');
    expect(statusWord('new')).toBe('New');
  });
  it('formats the match', () => {
    expect(matchText(92.4)).toBe('92%');
    expect(matchText(null)).toBe('Not scored');
  });
  it('handles a missing location', () => {
    expect(placeText({ location: null })).toBe('Location not listed');
  });
});

describe('prepareSummary', () => {
  it('lists what is left for the person', async () => {
    const { prepareSummary } = await import('./jobs');
    const t = prepareSummary({ filled: 5, missing: ['Visa sponsorship'], resume: 'Base resume' });
    expect(t).toContain('Filled 5 fields');
    expect(t).toContain('Visa sponsorship');
    expect(t).toContain('press Submit yourself');
  });
  it('mentions the cover letter when one is attached', async () => {
    const { prepareSummary } = await import('./jobs');
    const t = prepareSummary({ filled: 2, missing: [], resume: 'Tailored', cover_letter: 'Letter for Acme' });
    expect(t).toContain('Cover letter attached: Letter for Acme.');
  });
  it('says when there was no resume', async () => {
    const { prepareSummary } = await import('./jobs');
    expect(prepareSummary({ filled: 1, missing: [], resume: null })).toContain('No resume was attached');
  });
});

describe('byMatch', () => {
  it('puts the best matches first and keeps ties in order', () => {
    const jobs = [
      { id: 1, score: 40 },
      { id: 2, score: null },
      { id: 3, score: 90 },
      { id: 4, score: 40 }
    ];
    expect(byMatch(jobs).map((j) => j.id)).toEqual([3, 1, 4, 2]);
  });
  it('clamps odd scores', () => {
    expect(matchText(250)).toBe('100%');
  });
});

describe('knownText', () => {
  it('says how many people you know', () => {
    expect(knownText(0)).toBe('');
    expect(knownText(1)).toBe('You know 1 person');
    expect(knownText(4)).toBe('You know 4 people');
  });
});

describe('drafts', () => {
  it('notes what to check', () => {
    expect(draftNote([])).toContain('matches');
    expect(draftNote(['Google', '40%'])).toBe(
      'Check these, they are not in your profile or the posting: Google, 40%.'
    );
    expect(draftLabel('cover_letter')).toBe('cover letter');
    expect(draftLabel('tailored_resume')).toBe('tailored resume');
  });
});
