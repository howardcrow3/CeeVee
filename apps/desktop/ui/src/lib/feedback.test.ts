import { describe, expect, it } from 'vitest';
import { charsLeftText, feedbackInput, feedbackReady } from './feedback';

describe('feedback form', () => {
  it('needs stars, words or both', () => {
    expect(feedbackReady(0, '   ')).toBe(false);
    expect(feedbackReady(3, '')).toBe(true);
    expect(feedbackReady(0, 'Better cover letters')).toBe(true);
  });

  it('trims text and sends nulls for empty fields', () => {
    expect(feedbackInput(0, 'issue', '  Crash  ', ' ')).toEqual({ rating: null, category: 'issue', message: 'Crash', email: null });
    expect(feedbackInput(5, 'praise', '', 'a@b.co')).toEqual({ rating: 5, category: 'praise', message: '', email: 'a@b.co' });
  });

  it('counts down only near the limit', () => {
    expect(charsLeftText('x'.repeat(100))).toBe('');
    expect(charsLeftText('x'.repeat(4600))).toBe('400 characters left');
  });
});
