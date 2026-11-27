import { describe, expect, it } from 'vitest';
import { charsLeftText, feedbackEmailOk, feedbackInput, feedbackReady } from './feedback';

describe('feedback form', () => {
  it('needs stars, words or both', () => {
    expect(feedbackReady(0, '   ')).toBe(false);
    expect(feedbackReady(3, '')).toBe(true);
    expect(feedbackReady(0, 'Better cover letters')).toBe(true);
  });

  it('trims text and sends nulls for empty fields', () => {
    expect(feedbackInput(0, 'issue', '  Crash  ', ' ')).toEqual({ rating: null, category: 'issue', message: 'Crash', email: null, anonymous: false });
    expect(feedbackInput(5, 'praise', '', 'a@b.co')).toEqual({ rating: 5, category: 'praise', message: '', email: 'a@b.co', anonymous: false });
  });

  it('sends no email when anonymous', () => {
    expect(feedbackInput(3, 'other', 'hi', 'me@example.com', true)).toEqual({ rating: 3, category: 'other', message: 'hi', email: null, anonymous: true });
  });

  it('checks the email format like the service', () => {
    for (const ok of ['', '  ', 'a@b.co', 'first.last+tag@mail.example.com']) expect(feedbackEmailOk(ok)).toBe(true);
    for (const bad of ['nope', 'a@b', 'a@b.c', 'a b@c.co', 'a@b..co', '.a@b.co', 'a..b@c.co', 'a.@b.co', 'a@-b.co', 'a@b@c.co'])
      expect(feedbackEmailOk(bad), bad).toBe(false);
  });

  it('counts down only near the limit', () => {
    expect(charsLeftText('x'.repeat(100))).toBe('');
    expect(charsLeftText('x'.repeat(4600))).toBe('400 characters left');
  });
});
