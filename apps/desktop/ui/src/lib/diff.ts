export type DiffLine = { kind: 'same' | 'add' | 'del'; text: string };

/** The lines of `after` compared with `before`: lines in both, lines only in `after` (add) and lines only in `before` (del). */
export function diffLines(before: string, after: string): DiffLine[] {
  const a = before.split('\n').filter((l) => l.trim() !== '');
  const b = after.split('\n').filter((l) => l.trim() !== '');
  // Longest common subsequence table.
  const t: number[][] = Array.from({ length: a.length + 1 }, () => new Array(b.length + 1).fill(0));
  for (let i = a.length - 1; i >= 0; i--) {
    for (let j = b.length - 1; j >= 0; j--) {
      t[i][j] = a[i] === b[j] ? t[i + 1][j + 1] + 1 : Math.max(t[i + 1][j], t[i][j + 1]);
    }
  }
  const out: DiffLine[] = [];
  let i = 0;
  let j = 0;
  while (i < a.length && j < b.length) {
    if (a[i] === b[j]) {
      out.push({ kind: 'same', text: a[i] });
      i++;
      j++;
    } else if (t[i + 1][j] >= t[i][j + 1]) {
      out.push({ kind: 'del', text: a[i++] });
    } else {
      out.push({ kind: 'add', text: b[j++] });
    }
  }
  while (i < a.length) out.push({ kind: 'del', text: a[i++] });
  while (j < b.length) out.push({ kind: 'add', text: b[j++] });
  return out;
}

/** "3 lines added, 2 changed away" style summary of a diff. */
export function diffSummary(d: DiffLine[]): string {
  const add = d.filter((l) => l.kind === 'add').length;
  const del = d.filter((l) => l.kind === 'del').length;
  if (add === 0 && del === 0) return 'No changes from your base resume.';
  const one = (n: number, w: string) => `${n} ${w}${n === 1 ? '' : 's'}`;
  return `${one(add, 'line')} added or reworded, ${one(del, 'line')} replaced.`;
}

/** True when a line contains something the fact check flagged. */
export function isFlagged(line: string, flagged: string[]): boolean {
  const l = line.toLowerCase();
  return flagged.some((f) => f.trim() !== '' && l.includes(f.toLowerCase()));
}
