export const docKinds = [
  { value: 'cover_letter', label: 'Cover letter' },
  { value: 'base_resume', label: 'Base resume' },
  { value: 'resume_variant', label: 'Resume variant' },
  { value: 'email', label: 'Email' }
];

const labels: Record<string, string> = {
  ...Object.fromEntries(docKinds.map((k) => [k.value, k.label])),
  tailored_resume: 'Tailored resume'
};

export function kindLabel(kind: string): string {
  return labels[kind] ?? kind;
}

/** Resumes are stored as JSON; letters and emails as plain text. */
export function isResume(kind: string): boolean {
  return kind.endsWith('resume') || kind === 'resume_variant';
}

/** The starting text for a new document of this kind. */
export function starter(kind: string): string {
  return isResume(kind) ? '{\n  "basics": { "name": "" },\n  "work": []\n}' : '';
}
