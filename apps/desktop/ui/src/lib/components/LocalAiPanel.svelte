<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { commands } from '$lib/bindings';
  import { percent, sizeText } from '$lib/models';
  import type { ModelStatus, DownloadProgress } from '$lib/bindings';

  let model = $state<ModelStatus | null>(null);
  let progress = $state<DownloadProgress | null>(null);
  let timer: number | undefined;
  let ask = $state('');
  let answer = $state('');
  let asking = $state(false);
  let error = $state('');

  async function loadModel() {
    try {
      const m = await commands.modelStatus();
      if (m.status === 'ok') model = m.data;
      else error = m.error;
    } catch (e) {
      error = `Running outside the desktop app: ${String(e)}`;
    }
  }

  async function poll() {
    progress = await commands.modelProgress();
    if (!progress.running) {
      window.clearInterval(timer);
      timer = undefined;
      await loadModel();
    }
  }

  async function startDownload() {
    if (!model) return;
    error = '';
    const r = await commands.startModelDownload(model.id);
    if (r.status !== 'ok') {
      error = r.error;
      return;
    }
    timer = window.setInterval(poll, 1000);
    await poll();
  }

  async function tryModel() {
    asking = true;
    answer = '';
    error = '';
    const r = await commands.askModel(ask);
    if (r.status === 'ok') answer = r.data;
    else error = r.error;
    asking = false;
  }

  onDestroy(() => window.clearInterval(timer));
  onMount(loadModel);
</script>

<section>
  <h2>Local AI</h2>
  {#if model}
    <p class="note">
      This computer has {model.ram_gb} GB of memory. CeeVee recommends {model.name}
      ({sizeText(model.size_mb)}). It runs on your computer and nothing you write leaves it.
    </p>
    {#if model.installed}
      <p><span class="dot on"></span>{model.name} is installed.</p>
      <div class="row">
        <input aria-label="Ask the model" placeholder="Ask it something to try it out" bind:value={ask} />
        <button onclick={tryModel} disabled={asking || ask.trim() === ''}>{asking ? 'Thinking' : 'Ask'}</button>
      </div>
      {#if answer}<p role="status">{answer}</p>{/if}
    {:else if progress?.running}
      <p role="status">
        Downloading {sizeText(progress.downloaded_mb)} of {sizeText(progress.total_mb)}
        ({percent(progress.downloaded_mb, progress.total_mb)}%)
      </p>
      <div class="row"><button class="ghost" onclick={() => commands.cancelModelDownload()}>Cancel</button></div>
    {:else}
      <div class="row"><button onclick={startDownload}>Download {model.name}</button></div>
      {#if progress?.error}<p role="alert">{progress.error}</p>{/if}
    {/if}
  {/if}
</section>
{#if error}<p role="alert">{error}</p>{/if}

<style>
  section { display: flex; flex-direction: column; gap: 12px; }
  .row { display: flex; gap: 8px; flex-wrap: wrap; }
  .row input { flex: 1; min-width: 140px; }
</style>
