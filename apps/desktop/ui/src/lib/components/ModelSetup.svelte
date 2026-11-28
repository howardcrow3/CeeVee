<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { commands } from '$lib/bindings';
  import type { DownloadProgress, ModelStatus } from '$lib/bindings';
  import { percent, setupView, sizeText } from '$lib/models';

  /** Shown on a fresh install: offers the local AI download, with progress. Hidden once it is installed. */
  let { onready = () => {} }: { onready?: () => void } = $props();

  let model = $state<ModelStatus | null>(null);
  let progress = $state<DownloadProgress | null>(null);
  let loadError = $state('');
  let timer: number | undefined;
  let view = $derived(setupView(model, progress, loadError));

  async function load() {
    try {
      const m = await commands.modelStatus();
      if (m.status === 'ok') {
        model = m.data;
        loadError = '';
        if (m.data.installed) onready();
      } else loadError = m.error;
      progress = await commands.modelProgress();
      if (progress.running && timer === undefined) timer = window.setInterval(poll, 1000);
    } catch {
      // Running outside the desktop app: nothing to set up.
      model = { installed: true } as ModelStatus;
    }
  }

  async function poll() {
    progress = await commands.modelProgress();
    if (!progress.running) {
      window.clearInterval(timer);
      timer = undefined;
      await load();
    }
  }

  async function start() {
    if (!model) return;
    loadError = '';
    const r = await commands.startModelDownload(model.id);
    if (r.status !== 'ok') {
      loadError = r.error;
      return;
    }
    if (timer === undefined) timer = window.setInterval(poll, 1000);
    await poll();
  }

  onMount(load);
  onDestroy(() => window.clearInterval(timer));
</script>

{#if view !== 'ready' && view !== 'checking'}
  <section class="card" aria-label="Set up the local AI">
    {#if view === 'downloading' && progress}
      <strong>Downloading the local AI</strong>
      <progress max="100" value={percent(progress.downloaded_mb, progress.total_mb)}></progress>
      <span class="note" role="status">{sizeText(progress.downloaded_mb)} of {sizeText(progress.total_mb)} ({percent(progress.downloaded_mb, progress.total_mb)}%). You can keep using CeeVee meanwhile.</span>
      <div><button class="ghost" onclick={() => commands.cancelModelDownload()}>Cancel</button></div>
    {:else}
      <strong>CeeVee needs its local AI to chat</strong>
      {#if model}
        <span class="note">{model.name} ({sizeText(model.size_mb)}) runs on this computer, and nothing you write leaves it. Download it once and chat works.</span>
        <div><button onclick={start}>{view === 'failed' ? 'Try the download again' : `Download ${model.name}`}</button></div>
      {/if}
      {#if loadError || progress?.error}<span role="alert">{loadError || progress?.error}</span>{/if}
    {/if}
  </section>
{/if}

<style>
  .card { display: flex; flex-direction: column; gap: 8px; border: 1px solid var(--line); background: var(--surface); border-radius: var(--r-md); padding: 12px 16px; max-width: 720px; width: 100%; margin: 0 auto; box-sizing: border-box; }
  progress { width: 100%; }
</style>
