import type { CategoryLine } from '$lib/bindings';

/** Turn "a, b ,, c" into ["a", "b", "c"]. */
export function splitList(text: string): string[] {
  return text
    .split(/[,\n]/)
    .map((t) => t.trim())
    .filter((t) => t !== '');
}

/** Build a new category from what was typed into the form. */
export function newCategory(f: {
  name: string;
  titles: string;
  keywords: string;
  locations: string;
  avoid: string;
  remote_ok: boolean;
}): CategoryLine {
  return {
    id: 0,
    name: f.name.trim(),
    titles: splitList(f.titles),
    keywords: splitList(f.keywords),
    locations: splitList(f.locations),
    remote_ok: f.remote_ok,
    avoid: splitList(f.avoid),
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
  return parts.join('; ') || 'No rules yet';
}
