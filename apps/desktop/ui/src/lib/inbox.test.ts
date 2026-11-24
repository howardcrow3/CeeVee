import { describe, expect, it } from 'vitest';
import type { InboxLine } from './bindings';
import {
  dateWord,
  filterInbox,
  foundStatusWord,
  foundTitle,
  labelWord,
  looksLikeAddress,
  syncSummary,
  googleAccount
} from './inbox';

const line = (o: Partial<InboxLine>): InboxLine => ({
  id: 1,
  from: 'Dana',
  subject: 'Hello',
  summary: '',
  received_at: null,
  label: null,
  needs_reply: false,
  unread: false,
  has_draft: false,
  application_id: null,
  application: null,
  ...o
});

describe('inbox helpers', () => {
  it('words labels', () => {
    expect(labelWord('interview')).toBe('Interview');
    expect(labelWord('acknowledgement')).toBe('Received');
    expect(labelWord(null)).toBe('Not read yet');
    expect(labelWord('surprise')).toBe('Other');
  });
  it('filters', () => {
    const l = [line({ id: 1, needs_reply: true }), line({ id: 2, unread: true }), line({ id: 3 })];
    expect(filterInbox(l, 'reply').map((x) => x.id)).toEqual([1]);
    expect(filterInbox(l, 'unread').map((x) => x.id)).toEqual([2]);
    expect(filterInbox(l, 'all')).toHaveLength(3);
  });
  it('writes dates', () => {
    const now = new Date('2026-11-15T12:00:00Z');
    expect(dateWord('2026-11-13T10:00:00Z', now)).toBe('Nov 13');
    expect(dateWord('2025-02-01T10:00:00Z', now)).toBe('Feb 1 2025');
    expect(dateWord(null, now)).toBe('');
    expect(dateWord('nonsense', now)).toBe('');
  });
  it('summarises a sync in plain words', () => {
    expect(syncSummary({ employer: 0, status_changes: 0, new_jobs: 0, waiting_for_you: 0, drafts: 0 })).toBe('No new employer mail.');
    expect(syncSummary({ employer: 2, status_changes: 1, new_jobs: 0, waiting_for_you: 1, drafts: 2 })).toBe(
      '2 new employer messages, 1 application updated, 1 waiting for you in Applications, 2 replies drafted.'
    );
    expect(syncSummary({ employer: 3, status_changes: 0, new_jobs: 2, waiting_for_you: 0, drafts: 0 })).toBe(
      '3 new employer messages, 2 new jobs found to confirm.'
    );
  });
  it('words a job found in email', () => {
    expect(foundTitle({ company: 'Acme', title: 'Data Engineer' })).toBe('Data Engineer at Acme');
    expect(foundTitle({ company: 'Acme', title: 'Role not stated in email' })).toBe('Acme');
    expect(foundStatusWord('draft')).toBe('A recruiter reached out');
    expect(foundStatusWord('weird')).toBe('Found in email');
    expect(labelWord('inquiry')).toBe('Recruiter inquiry');
  });
  it('checks an address', () => {
    expect(looksLikeAddress('me@example.org')).toBe(true);
    expect(looksLikeAddress('me@')).toBe(false);
  });
});

describe('googleAccount', () => {
  it('finds the Gmail account so Settings can say signed in', () => {
    const imap = { provider: 'imap', address: 'a@b.co' };
    const gmail = { provider: 'gmail', address: 'me@gmail.com' };
    expect(googleAccount([imap])).toBeUndefined();
    expect(googleAccount([imap, gmail])).toBe(gmail);
    expect(googleAccount([])).toBeUndefined();
  });
});
