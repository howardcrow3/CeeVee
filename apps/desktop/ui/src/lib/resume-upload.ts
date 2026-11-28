/** The biggest resume file the app will read. */
export const MAX_RESUME_BYTES = 8 * 1024 * 1024;

/** The kinds of file the picker offers. */
export const RESUME_ACCEPT = '.pdf,.docx,.txt,.md,application/pdf,application/vnd.openxmlformats-officedocument.wordprocessingml.document,text/plain,text/markdown';

/** A sentence for a file that is too big, or null when the size is fine. */
export function sizeProblem(size: number): string | null {
  return size > MAX_RESUME_BYTES ? 'That file is too large for a resume. Choose a file under 8 MB.' : null;
}
