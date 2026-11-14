import { describe, expect, it } from 'vitest';
import { describeEdit, isResume, kindLabel, starter } from './documents';

describe('documents', () => {
  it('names kinds in plain words', () => {
    expect(kindLabel('cover_letter')).toBe('Cover letter');
    expect(kindLabel('tailored_resume')).toBe('Tailored resume');
    expect(kindLabel('odd')).toBe('odd');
  });
  it('knows which kinds are resumes', () => {
    expect(isResume('base_resume')).toBe(true);
    expect(isResume('resume_variant')).toBe(true);
    expect(isResume('tailored_resume')).toBe(true);
    expect(isResume('cover_letter')).toBe(false);
  });
  it('starts resumes with valid JSON', () => {
    expect(() => JSON.parse(starter('base_resume'))).not.toThrow();
    expect(starter('cover_letter')).toBe('');
  });
});

describe('describeEdit', () => {
  it('says when a change is saved cleanly', () => {
    expect(describeEdit({ applied: true, flagged: [] })).toContain('new version');
  });
  it('lists what was held back', () => {
    const t = describeEdit({ applied: false, flagged: ['40', 'Initech'] });
    expect(t).toContain('Held back');
    expect(t).toContain('Initech');
  });
  it('warns when flagged changes were accepted', () => {
    expect(describeEdit({ applied: true, flagged: ['40'] })).toContain('Check these');
  });
});
