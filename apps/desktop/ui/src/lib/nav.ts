/** The sections in the left navigation, in order. Settings is in the account menu. */
export const sections = [
  { href: '/', label: 'Chat' },
  { href: '/job-types', label: 'Job types' },
  { href: '/jobs', label: 'Jobs' },
  { href: '/applications', label: 'Applications' },
  { href: '/inbox', label: 'Inbox' },
  { href: '/network', label: 'Network' },
  { href: '/documents', label: 'Documents' }
] as const;

/** True when `path` is inside the section at `href`. */
export function isCurrent(href: string, path: string): boolean {
  return href === '/' ? path === '/' : path === href || path.startsWith(href + '/');
}
