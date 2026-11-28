import { commands } from './bindings';
import type { LicenseView } from './license';

/** The current license, shared by the account pill. Null until known or outside the desktop app. */
export const licenseUi = $state<{ view: LicenseView | null }>({ view: null });

export async function refreshLicense(): Promise<void> {
  try {
    const r = await commands.licenseStatus();
    if (r.status === 'ok') licenseUi.view = r.data;
  } catch {
    // Running outside the desktop app: no license line.
  }
}

export async function openBuyPage(): Promise<void> {
  try {
    await commands.openLicensePage('buy');
  } catch {
    // Running outside the desktop app.
  }
}
