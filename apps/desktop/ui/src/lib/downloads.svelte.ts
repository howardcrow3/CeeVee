import { commands } from '$lib/bindings';
import type { DownloadProgress, ModelStatus } from '$lib/bindings';
import { createDownloads } from '$lib/turtlebox/tb-downloads.js';
import type { DownloadItem } from '$lib/turtlebox/tb-downloads.js';
import { modelItem, type Intent } from '$lib/models';

/** The one download state for this window. The toolbar panel, the setup card and Settings all read it. */
export const downloads = createDownloads();

class DownloadView {
  items = $state<DownloadItem[]>([]);
  constructor() {
    downloads.subscribe((i) => (this.items = i));
  }
  get active() {
    return this.items.filter((i) => i.state === 'running' || i.state === 'queued').length;
  }
}
export const view = new DownloadView();

/** The local AI model: status, progress and the actions on it, shared by every screen. */
class ModelDownload {
  model = $state<ModelStatus | null>(null);
  progress = $state<DownloadProgress | null>(null);
  loadError = $state('');
  private intent: Intent = 'none';
  private seen = false;
  private timer: ReturnType<typeof setInterval> | undefined;
  private loading: Promise<void> | null = null;

  /** Read status and progress once. Safe to call from any screen. */
  load(): Promise<void> {
    this.loading ??= this.refresh().finally(() => (this.loading = null));
    return this.loading;
  }

  private async refresh() {
    try {
      const m = await commands.modelStatus();
      if (m.status === 'ok') {
        this.model = m.data;
        this.loadError = '';
      } else this.loadError = m.error;
      this.progress = await commands.modelProgress();
      if (this.progress.running) {
        this.seen = true;
        this.intent = 'none';
        this.poll();
      }
      this.publish();
    } catch {
      // Running outside the desktop app: nothing to download.
      this.model = { installed: true } as ModelStatus;
    }
  }

  private poll() {
    if (this.timer !== undefined) return;
    this.timer = setInterval(async () => {
      this.progress = await commands.modelProgress();
      this.publish();
      if (!this.progress.running) {
        clearInterval(this.timer);
        this.timer = undefined;
        await this.refresh();
      }
    }, 1000);
  }

  private publish() {
    const i = modelItem(this.model, this.progress, this.intent, this.seen);
    if (!i) return;
    downloads.update({
      ...i,
      actions: { pause: () => this.pause(), resume: () => this.start(), cancel: () => this.cancel(), retry: () => this.start() }
    });
  }

  async start() {
    if (!this.model) return;
    this.loadError = '';
    this.intent = 'none';
    const r = await commands.startModelDownload(this.model.id);
    if (r.status !== 'ok') {
      this.loadError = r.error;
      return;
    }
    this.seen = true;
    this.poll();
    this.progress = await commands.modelProgress();
    this.publish();
  }

  /** The backend keeps what it fetched, so pausing is a cancel that remembers you meant to come back. */
  async pause() {
    this.intent = 'pause';
    await commands.cancelModelDownload();
  }

  async cancel() {
    this.intent = 'cancel';
    await commands.cancelModelDownload();
  }
}
export const modelDownload = new ModelDownload();
