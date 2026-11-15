import type { LicenseView_Serialize } from './bindings';

export type LicenseView = LicenseView_Serialize;

/** One line for the banner, or null when nothing needs saying. */
export function bannerText(v: LicenseView): string | null {
  if (v.state === 'expired') {
    return 'Your 14-day trial has ended. CeeVee is read-only: you can still look at and export everything.';
  }
  if (v.state === 'trial' && v.days_left !== null) {
    if (v.days_left > 3) return null;
    return v.days_left <= 1
      ? 'Your trial ends today or tomorrow.'
      : `Your trial ends in ${v.days_left} days.`;
  }
  return null;
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
