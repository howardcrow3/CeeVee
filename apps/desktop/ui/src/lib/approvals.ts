/** Plain words for the action names tools use. Unknown names fall back to the name itself. */
const words: Record<string, string> = {
  'apply.submit': 'Submit an application',
  'mail.send': 'Send an email',
  'docs.save': 'Save a document'
};

export function actionWords(action: string): string {
  return words[action] ?? action;
}
