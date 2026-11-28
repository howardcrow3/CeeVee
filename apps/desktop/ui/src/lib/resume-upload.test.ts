import { describe, expect, it } from 'vitest';
import { MAX_RESUME_BYTES, RESUME_ACCEPT, sizeProblem } from './resume-upload';

describe('resume upload', () => {
  it('accepts PDF, Word, text and Markdown', () => {
    for (const ext of ['.pdf', '.docx', '.txt', '.md']) expect(RESUME_ACCEPT).toContain(ext);
  });
  it('rejects only oversized files', () => {
    expect(sizeProblem(MAX_RESUME_BYTES)).toBeNull();
    expect(sizeProblem(MAX_RESUME_BYTES + 1)).toMatch(/too large/);
  });
});
