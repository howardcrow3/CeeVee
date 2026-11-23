import type { JobLine } from './bindings';

/** The status word shown beside a job's dot. */
export function statusWord(state: string): string {
  return state === 'saved' ? 'Saved' : 'New';
}

/** Match as a whole-number percent. Scores run from 0 to 100. */
export function matchText(score: number | null | undefined): string {
  return score == null ? 'Not scored' : `${Math.max(0, Math.min(100, Math.round(score)))}%`;
}

/** Best matches first; jobs with the same match keep their order (newest first). */
export function byMatch<T extends { score?: number | null }>(jobs: T[]): T[] {
  return jobs
    .map((j, i) => ({ j, i }))
    .sort((a, b) => (b.j.score ?? -1) - (a.j.score ?? -1) || a.i - b.i)
    .map((x) => x.j);
}

export function placeText(j: Pick<JobLine, 'location'>): string {
  return j.location ?? 'Location not listed';
}

/** What to tell the person after CeeVee filled in an application form. */
export function prepareSummary(r: { filled: number; missing: string[]; resume: string | null; cover_letter?: string | null }): string {
  const fields = r.filled === 1 ? '1 field' : `${r.filled} fields`;
  const resume = r.resume ? ` Resume attached: ${r.resume}.` : ' No resume was attached, add one in Documents.';
  const letter = r.cover_letter ? ` Cover letter attached: ${r.cover_letter}.` : '';
  const missing = r.missing.length
    ? ` Still for you to answer: ${r.missing.join(', ')}.`
    : '';
  return `Filled ${fields}.${resume}${letter}${missing} Check the form in your browser and press Submit yourself.`;
}

/** "Know 3 people" next to a company, or nothing when you know no one there. */
export function knownText(n: number): string {
  if (n <= 0) return '';
  return n === 1 ? 'You know 1 person' : `You know ${n} people`;
}

/** What to say about a fresh draft: either clean, or listing what the person should check. */
export function draftNote(flagged: string[]): string {
  if (flagged.length === 0) return 'Everything in this draft matches your profile and the posting.';
  return `Check these, they are not in your profile or the posting: ${flagged.join(', ')}.`;
}

export function draftLabel(kind: string): string {
  return kind === 'cover_letter' ? 'cover letter' : 'tailored resume';
}
