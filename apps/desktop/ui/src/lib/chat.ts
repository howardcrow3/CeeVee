import { commands, type ApplicationLine, type JobLine } from './bindings';
import { statusLabel } from './applications';

export type Turn = { role: 'user' | 'assistant'; text: string };

export type ChatContext = {
  jobs: Pick<JobLine, 'title' | 'company' | 'score' | 'state'>[];
  applications: Pick<ApplicationLine, 'title' | 'company' | 'status'>[];
  jobTypes: { name: string; active: boolean }[];
  waiting: number;
};

const MAX_TURNS = 6;
const MAX_TURN_CHARS = 600;
const TOP_JOBS = 5;

/** A short, factual summary of the search that goes in front of every question. */
export function contextSummary(c: ChatContext): string {
  const lines: string[] = [];
  const types = c.jobTypes.filter((t) => t.active).map((t) => t.name);
  lines.push(types.length ? `Job types: ${types.join(', ')}.` : 'No job types are set up yet.');
  const fresh = c.jobs.filter((j) => j.state !== 'saved').length;
  const saved = c.jobs.length - fresh;
  lines.push(`Jobs found: ${c.jobs.length} (${fresh} new, ${saved} saved).`);
  const top = [...c.jobs]
    .filter((j) => j.score != null)
    .sort((a, b) => (b.score ?? 0) - (a.score ?? 0))
    .slice(0, TOP_JOBS)
    .map((j) => `${j.title}${j.company ? ` at ${j.company}` : ''} (${Math.round(j.score ?? 0)}%)`);
  if (top.length) lines.push(`Best matches: ${top.join('; ')}.`);
  if (c.applications.length) {
    const counts = new Map<string, number>();
    for (const a of c.applications) counts.set(a.status, (counts.get(a.status) ?? 0) + 1);
    const parts = [...counts].map(([s, n]) => `${n} ${statusLabel(s).toLowerCase()}`);
    lines.push(`Applications: ${c.applications.length} (${parts.join(', ')}).`);
  } else lines.push('No applications are tracked yet.');
  if (c.waiting > 0) lines.push(`${c.waiting} ${c.waiting === 1 ? 'item is' : 'items are'} waiting for approval.`);
  return lines.join('\n');
}

/** The short line under the composer saying what the assistant can see. */
export function contextChip(c: ChatContext): string {
  const j = c.jobs.length === 1 ? '1 job' : `${c.jobs.length} jobs`;
  const a = c.applications.length === 1 ? '1 application' : `${c.applications.length} applications`;
  return `Sees ${j}, ${a}`;
}

function clip(text: string): string {
  return text.length > MAX_TURN_CHARS ? `${text.slice(0, MAX_TURN_CHARS)}...` : text;
}

/** The full prompt: rules, what is known, the latest turns, then the new question. */
export function buildPrompt(summary: string, history: Turn[], message: string): string {
  const recent = history.slice(-MAX_TURNS).map((t) => `${t.role === 'user' ? 'Person' : 'CeeVee'}: ${clip(t.text)}`);
  return [
    'You are CeeVee, a private assistant that helps one person with their job search. Answer briefly and plainly.',
    'Use only the facts below. If the answer is not there, say you do not know. You cannot change anything in this answer. If asked to add or switch a job type, say to ask like "add a job type for product managers"; for anything else, say what the person can do in the app.',
    '',
    'Facts:',
    summary,
    '',
    ...(recent.length ? ['Conversation so far:', ...recent, ''] : []),
    `Person: ${clip(message)}`,
    'CeeVee:'
  ].join('\n');
}

/** What to say when a request fails because no model is installed or it stopped. */
export function friendlyError(e: string): { text: string; needsModel: boolean } {
  const needsModel = /download the model/i.test(e);
  return { text: needsModel ? 'The local AI is not installed yet.' : e, needsModel };
}

/** Starter questions, tailored to what exists. */
export function suggestions(c: ChatContext): string[] {
  const out: string[] = [];
  if (c.jobs.length > 0) out.push('Which jobs match me best?');
  if (c.applications.length > 0) out.push('How are my applications going?');
  if (c.waiting > 0) out.push('What is waiting for my approval?');
  out.push('What can you help me with?');
  return out;
}

/** What the person is looking at, appended to the facts so "this" has a meaning. */
export function focusLine(focus: string): string {
  const f = focus.trim();
  return f ? `\nThe person is looking at: ${f}` : '';
}

/** One job, described for the assistant. */
export function jobFocus(j: Pick<JobLine, 'title' | 'company' | 'score' | 'state'>): string {
  const at = j.company ? ` at ${j.company}` : '';
  const m = j.score != null ? `, ${Math.round(j.score)}% match` : '';
  return `the job ${j.title}${at} (${j.state}${m})`;
}

/** The facts the chat needs, read from the app. Empty when running outside it. */
export async function loadChatContext(): Promise<ChatContext> {
  const empty: ChatContext = { jobs: [], applications: [], jobTypes: [], waiting: 0 };
  try {
    const [j, a, c, w] = await Promise.all([commands.listJobs(), commands.listApplications(), commands.listCategories(), commands.pendingApprovalCount()]);
    return {
      jobs: j.status === 'ok' ? j.data : [],
      applications: a.status === 'ok' ? a.data : [],
      jobTypes: c.status === 'ok' ? c.data.map((x) => ({ name: x.name, active: x.active })) : [],
      waiting: w.status === 'ok' ? w.data : 0
    };
  } catch {
    return empty;
  }
}

/** True when the message asks CeeVee to change a job type, so it goes to the agent, not the plain Q&A. */
export function wantsAction(message: string): boolean {
  const m = message.toLowerCase();
  if (!/\bjob types?\b|\bsearch(es)? for\b|\blook(ing)? for\b/.test(m)) return false;
  return /\b(add|create|make|new|start|turn|switch|enable|disable|stop|pause|resume)\b/.test(m);
}

/** What to show under a reply from the agent. */
export function actionNotes(did: string[], waiting: number): string[] {
  const notes = [...did];
  if (waiting > 0) notes.push(`${waiting} ${waiting === 1 ? 'change is' : 'changes are'} waiting for your approval`);
  return notes;
}
