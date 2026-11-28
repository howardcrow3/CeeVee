/** Whole-number percent done, never above 100. */
export function percent(downloadedMb: number, totalMb: number): number {
  if (totalMb <= 0) return 0;
  return Math.min(100, Math.floor((downloadedMb / totalMb) * 100));
}

/** A size in megabytes as plain words, switching to gigabytes from 1000. */
export function sizeText(mb: number): string {
  return mb >= 1000 ? `${(mb / 1000).toFixed(1)} GB` : `${mb} MB`;
}

export type SetupView = 'checking' | 'ready' | 'offer' | 'downloading' | 'failed';

/** What the first-run model card should show, from the model status and download progress. */
export function setupView(
  model: { installed: boolean } | null,
  progress: { running: boolean; error: string } | null,
  loadError: string
): SetupView {
  if (model?.installed) return 'ready';
  if (progress?.running) return 'downloading';
  if (loadError || progress?.error) return 'failed';
  return model ? 'offer' : 'checking';
}
