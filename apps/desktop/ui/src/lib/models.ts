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

export type Intent = 'none' | 'pause' | 'cancel';

/**
 * The shared-download item for the local AI model, from what the backend reports.
 * Returns null when there is nothing to show yet (no download started and not installed from one).
 * `seen` is true once a download ran in this session, so an already-installed model is not listed as "done".
 */
export function modelItem(
  model: { id: string; name: string; installed: boolean } | null,
  progress: { running: boolean; downloaded_mb: number; total_mb: number; error: string } | null,
  intent: Intent,
  seen: boolean
): { id: string; name: string; note: string; total: number; done: number; state: 'running' | 'paused' | 'failed' | 'canceled' | 'done'; error: string } | null {
  if (!model || !progress) return null;
  const base = { id: `model:${model.id}`, name: model.name, note: 'local AI', total: progress.total_mb * 1e6, done: progress.downloaded_mb * 1e6, error: '' };
  if (progress.running) return { ...base, state: 'running' };
  if (model.installed) return seen ? { ...base, done: base.total, state: 'done' } : null;
  if (intent === 'pause') return { ...base, state: 'paused' };
  if (intent === 'cancel') return { ...base, state: 'canceled' };
  if (progress.error) return { ...base, state: 'failed', error: progress.error };
  return seen ? { ...base, state: 'failed', error: 'The download stopped' } : null;
}
