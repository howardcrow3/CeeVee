import type { LicenseView_Serialize } from './bindings';

export type LicenseView = LicenseView_Serialize;

export type PillTone = 'ok' | 'soon' | 'ended';

/** The short status under the account name in the bottom-left pill, or null until the license is known. */
export function pillText(v: LicenseView | null): { text: string; tone: PillTone } | null {
  if (!v) return null;
  if (v.state === 'licensed') return { text: 'Licensed', tone: 'ok' };
  if (v.state === 'expired') return { text: 'Trial ended · read-only', tone: 'ended' };
  if (v.days_left === null) return { text: 'Trial starting', tone: 'ok' };
  const text = `Trial · ${v.days_left} ${v.days_left === 1 ? 'day' : 'days'} left`;
  return { text, tone: v.days_left <= 3 ? 'soon' : 'ok' };
}

/** The license block at the top of the account menu. `buy` says whether to offer the Buy button. */
export function menuLicense(v: LicenseView | null): { title: string; detail: string; tone: PillTone; buy: boolean } | null {
  if (!v) return null;
  if (v.state === 'licensed') {
    return { title: 'Licensed', detail: `Works on up to ${v.seats ?? 2} computers.`, tone: 'ok', buy: false };
  }
  if (v.state === 'expired') {
    return {
      title: 'Your 14-day trial has ended',
      detail: 'CeeVee is read-only: you can still look at and export everything.',
      tone: 'ended',
      buy: true
    };
  }
  const left = v.days_left === null ? 'Your 14-day trial is starting.' : `${v.days_left} ${v.days_left === 1 ? 'day' : 'days'} left in your free trial.`;
  return { title: 'Free trial', detail: `${left} Everything is unlocked.`, tone: v.days_left !== null && v.days_left <= 3 ? 'soon' : 'ok', buy: true };
}

/** The status line shown in Settings. */
export function statusText(v: LicenseView): string {
  switch (v.state) {
    case 'licensed':
      return `Licensed (${v.key_hint ?? 'key saved'}). Works on up to ${v.seats ?? 2} computers.`;
    case 'expired':
      return 'The 14-day trial has ended. Enter a license key or buy one to keep going.';
    default:
      return v.days_left === null
        ? 'Your 14-day trial is starting.'
        : `Trial: ${v.days_left} ${v.days_left === 1 ? 'day' : 'days'} left. Everything is unlocked.`;
  }
}

/** True when the key box has something worth sending. */
export function canActivate(input: string): boolean {
  return input.replace(/[\s-]/g, '').length >= 8;
}

/** Make a pasted `ceevee://activate?key=…` link or a bare key into the key. */
export function keyFromInput(input: string): string {
  const t = input.trim();
  const m = /^ceevee:\/\/activate\/?\?(.*)$/i.exec(t);
  if (!m) return t;
  const key = new URLSearchParams(m[1]).get('key');
  return key ?? t;
}
