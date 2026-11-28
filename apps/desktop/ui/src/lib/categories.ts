import type { CategoryLine } from '$lib/bindings';

/** Turn "a, b ,, c" into ["a", "b", "c"]. */
export function splitList(text: string): string[] {
  return text
    .split(/[,\n]/)
    .map((t) => t.trim())
    .filter((t) => t !== '');
}

/** "120,000", "$120k" or "120 k" as dollars a year; 0 for blank or unreadable. */
export function payNumber(text: string): number {
  const t = text.toLowerCase().replace(/[$,\s]/g, '');
  const m = t.match(/^(\d+(?:\.\d+)?)(k?)$/);
  if (!m) return 0;
  const n = parseFloat(m[1]) * (m[2] === 'k' ? 1000 : 1);
  return n >= 1000 ? Math.round(n) : 0;
}

/** Build a new category from what was typed into the form. */
export function newCategory(f: {
  name: string;
  titles: string;
  keywords: string;
  locations: string;
  avoid: string;
  remote_ok: boolean;
  min_pay?: string;
  remote_only?: boolean;
}): CategoryLine {
  return {
    id: 0,
    name: f.name.trim(),
    titles: splitList(f.titles),
    keywords: splitList(f.keywords),
    locations: splitList(f.locations),
    remote_ok: f.remote_ok,
    avoid: splitList(f.avoid),
    min_pay: payNumber(f.min_pay ?? ''),
    remote_only: f.remote_only ?? false,
    active: true
  };
}

/** One line saying what a category looks for. */
export function describeCategory(c: CategoryLine): string {
  const parts: string[] = [];
  if (c.titles.length) parts.push(`titles: ${c.titles.join(', ')}`);
  if (c.keywords.length) parts.push(`mentions: ${c.keywords.join(', ')}`);
  const places = [...c.locations, ...(c.remote_ok ? ['remote'] : [])];
  if (places.length) parts.push(`places: ${places.join(', ')}`);
  if (c.avoid.length) parts.push(`avoids: ${c.avoid.join(', ')}`);
  if (c.remote_only) parts.push('remote only');
  if (c.min_pay > 0) parts.push(`pay at least $${c.min_pay.toLocaleString('en-US')}`);
  return parts.join('; ') || 'No rules yet';
}

/** Text for the edit form from a saved category. */
export function formFrom(c: CategoryLine) {
  return {
    name: c.name,
    titles: c.titles.join(', '),
    keywords: c.keywords.join(', '),
    locations: c.locations.join(', '),
    avoid: c.avoid.join(', '),
    remote_ok: c.remote_ok,
    min_pay: c.min_pay > 0 ? String(c.min_pay) : '',
    remote_only: c.remote_only
  };
}

/** The saved category with the edit form applied. Keeps its id and on/off state. */
export function edited(c: CategoryLine, f: ReturnType<typeof formFrom>): CategoryLine {
  return { ...newCategory(f), id: c.id, active: c.active };
}
