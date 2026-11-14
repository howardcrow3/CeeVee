import { describe, expect, it } from 'vitest';
import { isResume, kindLabel, starter } from './documents';

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
