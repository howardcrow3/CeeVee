export type DownloadState = 'queued' | 'running' | 'paused' | 'failed' | 'canceled' | 'done';
export interface DownloadActions { pause?: () => void; resume?: () => void; cancel?: () => void; retry?: () => void }
export interface DownloadItem {
  id: string; name: string; note?: string; total: number; done: number; state: DownloadState;
  error?: string; actions?: DownloadActions; speed: number; order: number; at: number; startedAt: number; finishedAt: number;
}
export type DownloadUpdate = Partial<DownloadItem> & { id: string };
export interface Downloads {
  update(next: DownloadUpdate): void;
  remove(id: string): void;
  clearCompleted(): void;
  get(id: string): DownloadItem | undefined;
  items(): DownloadItem[];
  activeCount(): number;
  groups(query?: string): { ongoing: DownloadItem[]; completed: DownloadItem[] };
  subscribe(fn: (items: DownloadItem[]) => void): () => void;
}
export const ACTIVE: string[];
export function formatBytes(bytes: number): string;
export function formatSpeed(bps: number): string;
export function formatEta(seconds: number): string;
export function percent(done: number, total: number): number;
export function statusLine(item: Partial<DownloadItem> & { state: DownloadState; done: number; total: number }): string;
export function matches(item: { name: string; note?: string }, query: string): boolean;
export function createDownloads(opts?: { now?: () => number }): Downloads;
