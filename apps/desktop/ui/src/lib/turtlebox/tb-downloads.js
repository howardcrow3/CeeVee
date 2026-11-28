// Vendored from howardcrow3/turtlebox-ui (js/tb-downloads.js). Do not edit by hand.
/* Turtlebox downloads: the state and wording behind the Downloads panel (docs/DOWNLOADS.md).
 * Dependency-free ES module. No DOM. An app feeds it progress, a view (css/downloads.css) draws it.
 * Sizes are in bytes everywhere. Speed is a moving average so the time left does not jump. */

export const ACTIVE = ['queued', 'running', 'paused'];

/** 31750000000 -> "31.75 GB". Decimal units, like the OS and the download sites. */
export function formatBytes(bytes) {
  const b = Math.max(0, Number(bytes) || 0);
  if (b >= 1e9) return `${trim(b / 1e9)} GB`;
  if (b >= 1e6) return `${trim(b / 1e6)} MB`;
  if (b >= 1e3) return `${Math.round(b / 1e3)} KB`;
  return `${Math.round(b)} B`;
}
function trim(n) { return n >= 100 ? String(Math.round(n)) : n.toFixed(2).replace(/0$/, '').replace(/\.0$/, '').replace(/\.$/, ''); }

/** 2000000 -> "2.00 MB/s" */
export function formatSpeed(bps) {
  const b = Math.max(0, Number(bps) || 0);
  if (b >= 1e6) return `${(b / 1e6).toFixed(2)} MB/s`;
  if (b >= 1e3) return `${Math.round(b / 1e3)} KB/s`;
  return `${Math.round(b)} B/s`;
}

/** 15840 -> "4 hrs 24 mins left". Under a minute: "less than a minute left". */
export function formatEta(seconds) {
  if (!Number.isFinite(seconds) || seconds < 0) return '';
  const m = Math.round(seconds / 60);
  if (seconds < 60) return 'less than a minute left';
  const h = Math.floor(m / 60), r = m % 60;
  if (h === 0) return `${m} ${m === 1 ? 'min' : 'mins'} left`;
  if (h >= 24) { const d = Math.floor(h / 24); return `${d} ${d === 1 ? 'day' : 'days'} ${h % 24} ${h % 24 === 1 ? 'hr' : 'hrs'} left`; }
  return r === 0 ? `${h} ${h === 1 ? 'hr' : 'hrs'} left` : `${h} ${h === 1 ? 'hr' : 'hrs'} ${r} ${r === 1 ? 'min' : 'mins'} left`;
}

/** Whole-number percent, 0 to 100. */
export function percent(done, total) {
  return total > 0 ? Math.min(100, Math.floor((done / total) * 100)) : 0;
}

/** The status line under the bar. */
export function statusLine(item) {
  const sizes = item.total > 0 ? `${formatBytes(item.done)} of ${formatBytes(item.total)}` : formatBytes(item.done);
  switch (item.state) {
    case 'running': {
      const parts = [sizes];
      if (item.speed > 0) parts.push(formatSpeed(item.speed));
      const eta = item.speed > 0 && item.total > 0 ? formatEta((item.total - item.done) / item.speed) : '';
      if (eta) parts.push(eta);
      return parts.join(' • ');
    }
    case 'queued': return 'Waiting to start';
    case 'paused': return `Paused • ${sizes}`;
    case 'failed': return item.error ? `Failed • ${item.error}` : 'Failed';
    case 'canceled': return 'Canceled';
    default: return item.total > 0 ? formatBytes(item.total) : sizes;
  }
}

/** Does the item match what was typed in the filter box? Case-insensitive, name or note. */
export function matches(item, query) {
  const q = String(query || '').trim().toLowerCase();
  return !q || `${item.name} ${item.note || ''}`.toLowerCase().includes(q);
}

/**
 * The shared download state for one app window.
 *   const dl = createDownloads({ now: () => Date.now() });
 *   dl.update({ id, name, total, done, state: 'running' });   // call as often as progress arrives
 *   dl.subscribe(fn)                                            // fn(snapshot) on every change
 * Actions (pause, resume, cancel, retry) are supplied per item as `actions`, so the panel
 * never needs to know how an app downloads: { pause, resume, cancel, retry } (all optional).
 */
export function createDownloads({ now = () => Date.now() } = {}) {
  const items = new Map();
  const subs = new Set();
  let order = 0;
  const snap = () => Array.from(items.values()).sort((a, b) => a.order - b.order);
  const emit = () => { const s = snap(); subs.forEach((f) => f(s)); };

  return {
    /** Add or change an item. Speed is worked out from how `done` moved since the last call. */
    update(next) {
      const t = now();
      const prev = items.get(next.id);
      const item = { note: '', error: '', actions: {}, ...prev, ...next };
      if (!prev) { item.order = order++; item.speed = 0; item.at = t; item.startedAt = t; }
      if (item.state === 'running' && prev && prev.state === 'running' && t > prev.at && item.done >= prev.done) {
        const inst = ((item.done - prev.done) / (t - prev.at)) * 1000;
        item.speed = prev.speed > 0 ? prev.speed * 0.7 + inst * 0.3 : inst;
      } else if (item.state !== 'running') item.speed = 0;
      item.at = t;
      if (!ACTIVE.includes(item.state) && !item.finishedAt) item.finishedAt = t;
      if (ACTIVE.includes(item.state)) item.finishedAt = 0;
      items.set(item.id, item);
      emit();
    },
    remove(id) { if (items.delete(id)) emit(); },
    /** Forget finished items (done, failed, canceled). */
    clearCompleted() {
      let n = 0;
      for (const [id, it] of items) if (!ACTIVE.includes(it.state)) { items.delete(id); n++; }
      if (n) emit();
    },
    get(id) { return items.get(id); },
    items: snap,
    /** Number for the badge on the toolbar button: downloads that are not finished. */
    activeCount() { return snap().filter((i) => i.state === 'running' || i.state === 'queued').length; },
    /** { ongoing, completed } after the filter. Completed lists newest first. */
    groups(query = '') {
      const all = snap().filter((i) => matches(i, query));
      return {
        ongoing: all.filter((i) => ACTIVE.includes(i.state)),
        completed: all.filter((i) => !ACTIVE.includes(i.state)).sort((a, b) => b.finishedAt - a.finishedAt),
      };
    },
    subscribe(fn) { subs.add(fn); fn(snap()); return () => subs.delete(fn); },
  };
}
