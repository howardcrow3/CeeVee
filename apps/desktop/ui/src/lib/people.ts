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
