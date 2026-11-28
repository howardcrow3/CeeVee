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

export type StepId = 'gmail' | 'model' | 'resume';
export type StepState = 'done' | 'current' | 'todo';

/** The three setup steps in order. The first one not done is the current step. */
export function stepStates(done: Record<StepId, boolean>): { id: StepId; state: StepState }[] {
  const order: StepId[] = ['gmail', 'model', 'resume'];
  const firstOpen = order.find((id) => !done[id]);
  return order.map((id) => ({ id, state: done[id] ? 'done' : id === firstOpen ? 'current' : 'todo' }));
}

/** How many of the three steps are finished, as "2 of 3". */
export function progressWord(done: Record<StepId, boolean>): string {
  const n = Object.values(done).filter(Boolean).length;
  return `${n} of 3 done`;
}

/** Company names from found jobs, each once, in the order first seen. */
export function companiesOf(found: { company: string }[]): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const j of found) {
    const name = j.company.trim();
    const key = name.toLowerCase();
    if (name && !seen.has(key)) {
      seen.add(key);
      out.push(name);
    }
  }
  return out;
}

/** The line that celebrates what the mail scan found. */
export function foundCelebration(found: { company: string }[]): string {
  const n = found.length;
  if (n === 0) {
    return 'You are connected. CeeVee found no job mail yet, and will pick up new replies as they arrive.';
  }
  const c = companiesOf(found).length;
  const jobs = n === 1 ? '1 job' : `${n} jobs`;
  const cos = c === 1 ? '1 company' : `${c} companies`;
  return `Nice. CeeVee found ${jobs} at ${cos} you are already in touch with.`;
}

/** The prompt that asks the local AI for resume feedback. Long resumes are trimmed to fit the model. */
export function resumeFeedbackPrompt(resume: string, maxChars = 6000): string {
  const text = resume.trim().slice(0, maxChars);
  return [
    'You are a careful career coach. Read this resume and give feedback in plain text.',
    'Start with one sentence on what is strongest. Then list at most 5 specific improvements, most useful first.',
    'Each improvement names the part of the resume it is about and says exactly what to change.',
    'Do not invent facts about the person.',
    '',
    'Resume:',
    text
  ].join('\n');
}

const KEY = 'ceevee.onboardingDone';

/** Whether the person has finished or skipped the walkthrough. Remembered between visits. */
export function onboardingFinished(): boolean {
  try {
    return localStorage.getItem(KEY) === '1';
  } catch {
    return false;
  }
}

export function markOnboardingFinished(): void {
  try {
    localStorage.setItem(KEY, '1');
  } catch {
    // Storage can be blocked; the walkthrough still works for this visit.
  }
}
