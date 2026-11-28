import type { PeopleImport } from '$lib/bindings';

export const tiers = [
  { value: 'close', label: 'Close' },
  { value: 'warm', label: 'Warm' },
  { value: 'weak', label: 'Weak' }
] as const;

export function tierLabel(value: string): string {
  return tiers.find((t) => t.value === value)?.label ?? value;
}

/** "Title at Company", whichever parts are known. */
export function workLine(title: string, company: string): string {
  if (title && company) return `${title} at ${company}`;
  return title || company;
}

/** One sentence for what a contacts import did. */
export function importSummary(s: PeopleImport): string {
  const parts = [`Added ${s.added} ${s.added === 1 ? 'person' : 'people'}`];
  if (s.matched > 0) parts.push(`filled in details for ${s.matched} you already had`);
  if (s.skipped > 0) parts.push(`skipped ${s.skipped} with no name`);
  return parts.join(', ') + '.';
}

/** "Not in touch since you added them" or "Last in touch about 3 months ago". */
export function silenceText(days: number, never: boolean): string {
  if (never) return 'Not in touch since you added them';
  if (days >= 365) return 'Last in touch over a year ago';
  if (days >= 60) return `Last in touch about ${Math.round(days / 30)} months ago`;
  return `Last in touch ${days} days ago`;
}

/** A `datetime-local` value (the person's own time zone) as UTC like `2026-12-03T17:00:00Z`. */
export function localToUtc(local: string): string {
  const d = new Date(local);
  if (Number.isNaN(d.getTime())) return '';
  return d.toISOString().replace(/\.\d{3}Z$/, 'Z');
}

/** A UTC time like `2026-12-03T17:00:00Z` in the person's own time zone, short. */
export function whenText(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString(undefined, { weekday: 'short', month: 'short', day: 'numeric', hour: 'numeric', minute: '2-digit' });
}

/** "2 of 3 this week", or a nudge when the goal is met. */
export function goalText(done: number, goal: number): string {
  if (done >= goal) return `Goal met: ${done} ${done === 1 ? 'person' : 'people'} this week.`;
  return `${done} of ${goal} this week`;
}

/** The introduction results in one line, or empty when nothing was asked. */
export function funnelText(asked: number, replied: number, referred: number): string {
  if (asked === 0) return '';
  return `Last 30 days: asked ${asked}, ${replied} answered, ${referred} referred.`;
}

/** "Moved from Acme to Globex as Engineer", whichever parts are known. */
export function moveText(from: string, to: string, title: string): string {
  const where = to ? ` to ${to}${title ? ` as ${title}` : ''}` : '';
  return `Moved${from ? ` from ${from}` : ''}${where}`;
}
