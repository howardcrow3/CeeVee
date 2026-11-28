import { describe, expect, it } from 'vitest';
import { accepted, savedSummary, companiesOf, foundCelebration, progressWord, resumeFeedbackPrompt, stepStates } from './onboarding';

describe('onboarding', () => {
  it('keeps only the ticked changes in order', () => {
    const cs = [
      { change: 'a', text: 'A' },
      { change: 'b', text: 'B' },
      { change: 'c', text: 'C' }
    ];
    expect(accepted(cs, [true, false, true])).toEqual(['a', 'c']);
    expect(accepted(cs, [])).toEqual([]);
  });

  it('says what was saved', () => {
    expect(savedSummary(0)).toBe('Nothing was changed.');
    expect(savedSummary(1)).toBe('Saved 1 change to your profile.');
    expect(savedSummary(4)).toBe('Saved 4 changes to your profile.');
  });
});

describe('three steps', () => {
  it('marks the first open step as current', () => {
    expect(stepStates({ gmail: true, model: false, resume: false }).map((s) => s.state)).toEqual(['done', 'current', 'todo']);
    expect(stepStates({ gmail: false, model: true, resume: true }).map((s) => s.state)).toEqual(['current', 'done', 'done']);
    expect(stepStates({ gmail: true, model: true, resume: true }).every((s) => s.state === 'done')).toBe(true);
  });
  it('counts finished steps', () => {
    expect(progressWord({ gmail: true, model: false, resume: true })).toBe('2 of 3 done');
  });
  it('lists each company once', () => {
    expect(companiesOf([{ company: 'Acme' }, { company: 'acme ' }, { company: 'Beta' }, { company: '' }])).toEqual(['Acme', 'Beta']);
  });
  it('celebrates what was found', () => {
    expect(foundCelebration([])).toMatch(/found no job mail/);
    expect(foundCelebration([{ company: 'A' }])).toBe('Nice. CeeVee found 1 job at 1 company you are already in touch with.');
    expect(foundCelebration([{ company: 'A' }, { company: 'A' }, { company: 'B' }])).toContain('3 jobs at 2 companies');
  });
  it('builds a bounded feedback prompt', () => {
    const p = resumeFeedbackPrompt('x'.repeat(10000), 100);
    expect(p).toContain('Resume:');
    expect(p.endsWith('x'.repeat(100))).toBe(true);
    expect(p.length).toBeLessThan(700);
  });
});
