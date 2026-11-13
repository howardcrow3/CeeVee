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
