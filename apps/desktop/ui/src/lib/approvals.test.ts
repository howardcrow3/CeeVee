import { describe, expect, it } from 'vitest';
import { actionWords } from './approvals';

describe('actionWords', () => {
  it('uses plain words for known actions', () => {
    expect(actionWords('apply.submit')).toBe('Submit an application');
  });
  it('falls back to the name', () => {
    expect(actionWords('x.y')).toBe('x.y');
  });
});
