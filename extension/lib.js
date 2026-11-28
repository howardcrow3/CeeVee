// Pure helpers shared by the content script and the tests. No page access in here.
(function (root) {
  /** Tidy a label: one space between words, no trailing asterisk or "(required)". */
  function cleanLabel(s) {
    return String(s || '')
      .replace(/\s+/g, ' ')
      .replace(/\(required\)/gi, '')
      .replace(/[*✱]+\s*$/u, '')
      .trim();
  }

  /** The first usable label out of the places a form puts it, best first. */
  function pickLabel(parts) {
    const order = ['labelText', 'ariaLabelledBy', 'ariaLabel', 'legend', 'placeholder', 'name'];
    for (const k of order) {
      const v = cleanLabel(parts[k]);
      if (v) return v;
    }
    return '';
  }

  /**
   * What kind of control this is for CeeVee, or null when it is left alone: hidden fields,
   * buttons, radio buttons and anything that submits.
   */
  function controlKind(tag, type) {
    tag = String(tag || '').toLowerCase();
    type = String(type || 'text').toLowerCase();
    if (tag === 'textarea') return 'textarea';
    if (tag === 'select') return 'select';
    if (tag !== 'input') return null;
    if (type === 'file') return 'file';
    if (type === 'checkbox') return 'checkbox';
    if (['text', 'email', 'tel', 'url', 'search', 'number', ''].includes(type)) return 'text';
    return null;
  }

  /** How many of each state the checklist has, for the one-line summary. */
  function summarize(checklist) {
    const n = { filled: 0, check_this: 0, needs_you: 0, left_blank: 0 };
    for (const i of checklist || []) if (i.state in n) n[i.state] += 1;
    return n;
  }

  function summaryText(checklist, ready) {
    const n = summarize(checklist);
    const parts = [];
    if (n.filled + n.check_this > 0) parts.push(`${n.filled + n.check_this} filled`);
    if (n.needs_you > 0) parts.push(`${n.needs_you} need${n.needs_you === 1 ? 's' : ''} you`);
    if (n.check_this > 0) parts.push(`${n.check_this} to check`);
    if (n.left_blank > 0) parts.push(`${n.left_blank} left blank`);
    const line = parts.join(', ') || 'Nothing to fill on this page';
    return ready ? `${line}. Ready for you to review and submit.` : `${line}.`;
  }

  /** The addresses where the app may be listening. */
  const FIRST_PORT = 47831;
  const PORT_COUNT = 10;
  function candidatePorts() {
    return Array.from({ length: PORT_COUNT }, (_, i) => FIRST_PORT + i);
  }

  const lib = { cleanLabel, pickLabel, controlKind, summarize, summaryText, candidatePorts };
  if (typeof module !== 'undefined' && module.exports) module.exports = lib;
  else root.CeeVeeLib = lib;
})(typeof globalThis !== 'undefined' ? globalThis : this);
