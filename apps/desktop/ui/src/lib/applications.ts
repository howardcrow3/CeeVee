import type { ApplicationLine } from '$lib/bindings';

export const statuses = [
  { value: 'draft', label: 'Draft' },
  { value: 'sent', label: 'Applied' },
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

const openStages = ['draft', 'sent', 'screening', 'interview', 'offer'] as const;

export type BoardColumn = { key: string; label: string; total: number; cards: ApplicationLine[]; more: number };

/**
 * Board columns: one per open stage plus "Closed" for everything that ended.
 * Each column shows at most `limit` cards (all of them when `expanded` has its key) and counts the rest.
 */
export function boardColumns(items: ApplicationLine[], limit = 5, expanded: ReadonlySet<string> = new Set()): BoardColumn[] {
  const cols = [
    ...openStages.map((s) => ({ key: s as string, label: statusLabel(s), all: items.filter((i) => i.status === s) })),
    { key: 'closed', label: 'Closed', all: items.filter((i) => !(openStages as readonly string[]).includes(i.status)) }
  ];
  return cols.map((c) => {
    const cards = expanded.has(c.key) ? c.all : c.all.slice(0, limit);
    return { key: c.key, label: c.label, total: c.all.length, cards, more: c.all.length - cards.length };
  });
}

/** The status a card takes when dropped on a board column, or null when nothing should change. */
export function dropStatus(columnKey: string, current: string): string | null {
  if (!(openStages as readonly string[]).includes(columnKey)) return null;
  return columnKey === current ? null : columnKey;
}
