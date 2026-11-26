/** The "Send feedback" form's rules. Sending and signing happen in Rust (`send_feedback`). */

export const feedbackCategories = [
  { value: 'feature', label: 'Idea' },
  { value: 'issue', label: 'Problem' },
  { value: 'praise', label: 'Praise' },
  { value: 'other', label: 'Other' }
] as const;

export type FeedbackCategory = (typeof feedbackCategories)[number]['value'];

export const MAX_FEEDBACK = 5000;

/** Send is allowed once there is a star rating, some words, or both. */
export function feedbackReady(rating: number, message: string): boolean {
  return rating > 0 || message.trim().length > 0;
}

/** The command input: trimmed text, and nulls for what the user left empty. */
export function feedbackInput(rating: number, category: FeedbackCategory, message: string, email: string) {
  return {
    rating: rating > 0 ? rating : null,
    category,
    message: message.trim(),
    email: email.trim() || null
  };
}

/** Shown under the text box near the limit, empty otherwise. */
export function charsLeftText(message: string): string {
  const left = MAX_FEEDBACK - message.length;
  return left <= 500 ? `${left} characters left` : '';
}
