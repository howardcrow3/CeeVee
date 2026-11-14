import type { JobLine } from './bindings';

/** The status word shown beside a job's dot. */
export function statusWord(state: string): string {
  return state === 'saved' ? 'Saved' : 'New';
}

/** Match as a whole-number percent, or an em-free dash when the job has not been scored. */
export function matchText(score: number | null | undefined): string {
  return score == null ? 'Not scored' : `${Math.round(score * 100)}%`;
}

export function placeText(j: Pick<JobLine, 'location'>): string {
  return j.location ?? 'Location not listed';
}
