export type ThemeChoice = 'system' | 'light' | 'dark';

const KEY = 'ceevee.theme';

/** Parse a stored value, falling back to following the system. */
export function parseTheme(v: string | null | undefined): ThemeChoice {
  return v === 'light' || v === 'dark' ? v : 'system';
}

/** Force light or dark with `data-theme`, or remove it to follow the system. */
export function applyTheme(choice: ThemeChoice, root: { setAttribute(k: string, v: string): void; removeAttribute(k: string): void }): void {
  if (choice === 'system') root.removeAttribute('data-theme');
  else root.setAttribute('data-theme', choice);
}

export function loadTheme(): ThemeChoice {
  try {
    return parseTheme(localStorage.getItem(KEY));
  } catch {
    return 'system';
  }
}

export function saveTheme(choice: ThemeChoice): void {
  try {
    if (choice === 'system') localStorage.removeItem(KEY);
    else localStorage.setItem(KEY, choice);
  } catch {
    // Storage unavailable; the choice lasts until the app closes.
  }
}
