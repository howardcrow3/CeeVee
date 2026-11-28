import { tabOrDefault, type SettingsTab } from './settings';

/** Whether the settings modal is open and which tab it shows. */
export const settingsUi = $state<{ open: boolean; tab: SettingsTab }>({ open: false, tab: 'general' });

export function openSettings(tab?: string): void {
  settingsUi.tab = tabOrDefault(tab);
  settingsUi.open = true;
}
export function closeSettings(): void {
  settingsUi.open = false;
}
