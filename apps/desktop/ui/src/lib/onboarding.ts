import type { ProfileChange } from '$lib/bindings';

/** The changes the person left ticked, in their original order. */
export function accepted(changes: ProfileChange[], ticked: boolean[]): string[] {
  return changes.filter((_, i) => ticked[i]).map((c) => c.change);
}

/** One sentence saying what happened after saving. */
export function savedSummary(n: number): string {
  if (n === 0) return 'Nothing was changed.';
  return n === 1 ? 'Saved 1 change to your profile.' : `Saved ${n} changes to your profile.`;
}
