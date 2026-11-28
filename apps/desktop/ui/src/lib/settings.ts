/** The tabs of the settings modal, in order. Add a tab here and map it to a component in SettingsModal. */
export const settingsTabs = [
  { id: 'general', label: 'General' },
  { id: 'search', label: 'Job search' },
  { id: 'accounts', label: 'Accounts' },
  { id: 'browser', label: 'Browser' },
  { id: 'privacy', label: 'Privacy and about' }
] as const;

export type SettingsTab = (typeof settingsTabs)[number]['id'];

/** A known tab id, or the first tab for anything else. */
export function tabOrDefault(id: string | null | undefined): SettingsTab {
  return settingsTabs.find((t) => t.id === id)?.id ?? settingsTabs[0].id;
}
