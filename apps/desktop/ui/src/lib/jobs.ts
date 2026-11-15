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

/** What to tell the person after CeeVee filled in an application form. */
export function prepareSummary(r: { filled: number; missing: string[]; resume: string | null }): string {
  const fields = r.filled === 1 ? '1 field' : `${r.filled} fields`;
  const resume = r.resume ? ` Resume attached: ${r.resume}.` : ' No resume was attached, add one in Documents.';
  const missing = r.missing.length
    ? ` Still for you to answer: ${r.missing.join(', ')}.`
    : '';
  return `Filled ${fields}.${resume}${missing} Check the form in your browser and press Submit yourself.`;
}
