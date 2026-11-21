import type { AnalyticsView } from './bindings';

/** Shown once, at first run, until the person answers either way. */
export function shouldAsk(v: AnalyticsView | null): boolean {
  return v !== null && v.consent === 'unset';
}

/** What is and is not sent, in one place so the card, Settings and the privacy notice agree. */
export const sentSummary =
  'Only anonymous counts: the app version, your operating system, which features you use, and whether CeeVee closed unexpectedly. Never your resume, jobs, email, chats or anything you type.';

export function statusLine(v: AnalyticsView): string {
  if (v.consent === 'granted') return v.available ? 'Sharing anonymous usage counts.' : 'On, but this build has no analytics destination, so nothing is sent.';
  if (v.consent === 'denied') return 'Off. Nothing is sent.';
  return 'Not chosen yet. Nothing is sent.';
}
