import type { BoardLine } from '$lib/bindings';

export const boardKinds = [
  { value: 'greenhouse', label: 'Greenhouse' },
  { value: 'lever', label: 'Lever' }
];

/** Drop rows left completely blank and trim the rest. */
export function cleanBoards(rows: BoardLine[]): BoardLine[] {
  return rows
    .map((r) => ({ kind: r.kind, token: r.token.trim(), company: r.company.trim() }))
    .filter((r) => r.token !== '' || r.company !== '');
}

/** Say what a search found in one plain sentence. */
export function searchSummary(r: {
  boards_ok: number;
  new: number;
  failures: string[];
}): string {
  const found = r.new === 1 ? '1 new job' : `${r.new} new jobs`;
  const boards = r.boards_ok === 1 ? '1 company' : `${r.boards_ok} companies`;
  const failed = r.failures.length ? ` ${r.failures.length} could not be read.` : '';
  return `Found ${found} at ${boards}.${failed}`;
}

/** A starter list of well-known US employers on the two supported job boards. Names that no longer
 * exist simply show up as "could not be read" after a search, and can be removed. */
export const starterBoards: BoardLine[] = [
  { kind: 'greenhouse', token: 'stripe', company: 'Stripe' },
  { kind: 'greenhouse', token: 'airbnb', company: 'Airbnb' },
  { kind: 'greenhouse', token: 'discord', company: 'Discord' },
  { kind: 'greenhouse', token: 'figma', company: 'Figma' },
  { kind: 'greenhouse', token: 'reddit', company: 'Reddit' },
  { kind: 'greenhouse', token: 'databricks', company: 'Databricks' },
  { kind: 'greenhouse', token: 'coinbase', company: 'Coinbase' },
  { kind: 'lever', token: 'palantir', company: 'Palantir' },
  { kind: 'lever', token: 'spotify', company: 'Spotify' },
  { kind: 'lever', token: 'plaid', company: 'Plaid' }
];

/** Add the starter companies that are not already in the list (same board type and name). */
export function withStarters(rows: BoardLine[]): BoardLine[] {
  const have = new Set(rows.map((r) => `${r.kind}:${r.token.trim().toLowerCase()}`));
  const extra = starterBoards.filter((s) => !have.has(`${s.kind}:${s.token}`));
  return [...rows, ...extra];
}
