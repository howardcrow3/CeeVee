import type { ApplicationLine } from '$lib/bindings';

export const statuses = [
  { value: 'draft', label: 'Draft' },
  { value: 'sent', label: 'Sent' },
  { value: 'screening', label: 'Screening' },
  { value: 'interview', label: 'Interview' },
  { value: 'offer', label: 'Offer' },
  { value: 'accepted', label: 'Accepted' },
  { value: 'rejected', label: 'Rejected' },
  { value: 'declined', label: 'Declined' },
  { value: 'withdrawn', label: 'Withdrawn' }
] as const;

export function statusLabel(value: string): string {
  return statuses.find((s) => s.value === value)?.label ?? value;
}

/** Applications grouped by status in pipeline order; empty groups are left out. */
export function groupByStatus(items: ApplicationLine[]): { status: string; items: ApplicationLine[] }[] {
  return statuses
    .map((s) => ({ status: s.value as string, items: items.filter((i) => i.status === s.value) }))
    .filter((g) => g.items.length > 0);
}

/** "Title at Company", or just the title when the company is unknown. */
export function applicationName(a: ApplicationLine): string {
  return a.company ? `${a.title} at ${a.company}` : a.title;
}
