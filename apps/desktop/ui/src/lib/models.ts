/** Whole-number percent done, never above 100. */
export function percent(downloadedMb: number, totalMb: number): number {
  if (totalMb <= 0) return 0;
  return Math.min(100, Math.floor((downloadedMb / totalMb) * 100));
}

/** A size in megabytes as plain words, switching to gigabytes from 1000. */
export function sizeText(mb: number): string {
  return mb >= 1000 ? `${(mb / 1000).toFixed(1)} GB` : `${mb} MB`;
}
