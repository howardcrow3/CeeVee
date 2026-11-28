import type { InboxLine } from './bindings';

const labelWords: Record<string, string> = {
  interview: 'Interview',
  assessment: 'Assessment',
  offer: 'Offer',
  rejection: 'Rejection',
  question: 'Question',
  acknowledgement: 'Received',
  inquiry: 'Recruiter inquiry',
  other: 'Other'
};

/** The word shown for a label; unlabelled mail is "Not read yet". */
export function labelWord(label: string | null | undefined): string {
  return label ? (labelWords[label] ?? 'Other') : 'Not read yet';
}

export type InboxFilter = 'all' | 'reply' | 'unread';

export function filterInbox(lines: InboxLine[], f: InboxFilter): InboxLine[] {
  if (f === 'reply') return lines.filter((l) => l.needs_reply);
  if (f === 'unread') return lines.filter((l) => l.unread);
  return lines;
}

/** `13 Nov` for a timestamp in this year, `13 Nov 2025` otherwise. */
export function dateWord(iso: string | null | undefined, now = new Date()): string {
  if (!iso) return '';
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '';
  const day = d.toLocaleDateString('en-US', { day: 'numeric', month: 'short' });
  return d.getFullYear() === now.getFullYear() ? day : `${day} ${d.getFullYear()}`;
}

/** What to tell the person after a sync. */
export function syncSummary(s: {
  employer: number;
  status_changes: number;
  new_jobs: number;
  waiting_for_you: number;
  drafts: number;
  seen?: number;
}): string {
  if (s.employer === 0) {
    if (!s.seen) return 'No new employer mail. Your mailbox returned no recent messages.';
    const n = s.seen === 1 ? '1 recent message' : `${s.seen} recent messages`;
    return `No new employer mail. Looked at ${n}; none looked like they were about a job.`;
  }
  const parts = [s.employer === 1 ? '1 new employer message' : `${s.employer} new employer messages`];
  if (s.new_jobs) parts.push(s.new_jobs === 1 ? '1 new job found to confirm' : `${s.new_jobs} new jobs found to confirm`);
  if (s.status_changes) parts.push(s.status_changes === 1 ? '1 application updated' : `${s.status_changes} applications updated`);
  if (s.waiting_for_you) parts.push(`${s.waiting_for_you} waiting for you in Applications`);
  if (s.drafts) parts.push(s.drafts === 1 ? '1 reply drafted' : `${s.drafts} replies drafted`);
  return parts.join(', ') + '.';
}

/** The address with its provider's usual server, or an empty host when it is not known. */
export function looksLikeAddress(a: string): boolean {
  return /^[^@\s]+@[^@\s]+\.[^@\s]+$/.test(a.trim());
}

/** The connected Google (Gmail) account, if any, for the "Signed in" line in Settings. */
export function googleAccount<T extends { provider: string }>(accounts: T[]): T | undefined {
  return accounts.find((a) => a.provider === 'gmail');
}

const foundStatus: Record<string, string> = {
  draft: 'A recruiter reached out',
  sent: 'You applied',
  screening: 'Assessment',
  interview: 'Interview',
  offer: 'Offer',
  rejected: 'Rejected'
};

/** What the email that started a found job said, in the person's words. */
export function foundStatusWord(status: string): string {
  return foundStatus[status] ?? 'Found in email';
}

/** `Role at Company`, or just the company when the email did not name the role. */
export function foundTitle(j: { company: string; title: string }): string {
  const noRole = j.title === '' || j.title.startsWith('Role not stated');
  return noRole ? j.company : `${j.title} at ${j.company}`;
}

/** True when a mail check failed because the saved sign-in is missing, expired or refused. */
export function needsSignIn(error: string): boolean {
  return /connect (google|the account) again|missing from the keychain|refused the sign-in|invalid_grant|token (has been )?(expired|revoked)|not signed in/i.test(error);
}
