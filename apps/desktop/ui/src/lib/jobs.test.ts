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

import { filterJobs } from './jobs';

describe('filterJobs', () => {
  const jobs = [
    { id: 1, title: 'Product Manager', company: 'Acme', location: 'Denver', state: 'new' },
    { id: 2, title: 'Designer', company: 'Globex', location: null, state: 'saved' }
  ];
  it('keeps everything with no filter', () => {
    expect(filterJobs(jobs, { state: 'all', text: '' })).toHaveLength(2);
  });
  it('filters by status', () => {
    expect(filterJobs(jobs, { state: 'saved', text: '' }).map((j) => j.title)).toEqual(['Designer']);
    expect(filterJobs(jobs, { state: 'new', text: '' }).map((j) => j.title)).toEqual(['Product Manager']);
  });
  it('matches every typed word across title, company and place', () => {
    expect(filterJobs(jobs, { state: 'all', text: 'acme denver' })).toHaveLength(1);
    expect(filterJobs(jobs, { state: 'all', text: 'acme tokyo' })).toHaveLength(0);
  });
});

import { selectedText, stepId, toggleAllIds, toggleId } from './jobs';

describe('selection', () => {
  it('adds and removes one id without changing the original', () => {
    const a = new Set([1]);
    const b = toggleId(a, 2);
    expect([...b].sort()).toEqual([1, 2]);
    expect([...a]).toEqual([1]);
    expect([...toggleId(b, 1)]).toEqual([2]);
  });
  it('selects all, then none', () => {
    const all = toggleAllIds(new Set(), [1, 2, 3]);
    expect(all.size).toBe(3);
    expect(toggleAllIds(all, [1, 2, 3]).size).toBe(0);
    expect(toggleAllIds(new Set(), []).size).toBe(0);
  });
  it('steps through the list and stops at the ends', () => {
    expect(stepId([4, 5, 6], null, 1)).toBe(4);
    expect(stepId([4, 5, 6], null, -1)).toBe(6);
    expect(stepId([4, 5, 6], 5, 1)).toBe(6);
    expect(stepId([4, 5, 6], 6, 1)).toBe(6);
    expect(stepId([4, 5, 6], 4, -1)).toBe(4);
    expect(stepId([], 1, 1)).toBeNull();
  });
  it('labels the bulk bar', () => expect(selectedText(3)).toBe('3 selected'));
});

import { jobsOfType } from './jobs';

describe('by job type', () => {
  const jobs = [
    { id: 1, title: 'PM', company: 'A', location: null, state: 'new' },
    { id: 2, title: 'Designer', company: 'B', location: null, state: 'new' }
  ];
  const matches = [{ job: 1, job_type: 10 }, { job: 2, job_type: 20 }];
  it('keeps only the jobs of the chosen type', () => {
    expect(filterJobs(jobs, { state: 'all', text: '', only: jobsOfType(matches, 20) }).map((j) => j.id)).toEqual([2]);
  });
  it('shows everything for all job types', () => {
    expect(jobsOfType(matches, null)).toBeNull();
    expect(filterJobs(jobs, { state: 'all', text: '', only: null })).toHaveLength(2);
  });
});
