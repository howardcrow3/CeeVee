import type { InboxLine } from './bindings';

const labelWords: Record<string, string> = {
  interview: 'Interview',
  assessment: 'Assessment',
  offer: 'Offer',
  rejection: 'Rejection',
  question: 'Question',
  acknowledgement: 'Received',
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
  waiting_for_you: number;
  drafts: number;
  seen?: number;
}): string {
  if (s.employer === 0) {
    if (!s.seen) return 'No new employer mail. Your mailbox returned no recent messages.';
    const n = s.seen === 1 ? '1 recent message' : `${s.seen} recent messages`;
    return `No new employer mail. Looked at ${n}; none named a company you applied to or came from a job system.`;
  }
  const parts = [s.employer === 1 ? '1 new employer message' : `${s.employer} new employer messages`];
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
