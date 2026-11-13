/** The sections in the left navigation, in order. */
export const sections = [
  { href: '/', label: 'Today' },
  { href: '/jobs', label: 'Jobs' },
  { href: '/applications', label: 'Applications' },
  { href: '/inbox', label: 'Inbox' },
  { href: '/network', label: 'Network' },
  { href: '/documents', label: 'Documents' },
  { href: '/settings', label: 'Settings' }
] as const;

/** True when `path` is inside the section at `href`. */
export function isCurrent(href: string, path: string): boolean {
  return href === '/' ? path === '/' : path === href || path.startsWith(href + '/');
}
