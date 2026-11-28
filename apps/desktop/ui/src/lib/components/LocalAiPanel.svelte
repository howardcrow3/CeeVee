<script lang="ts">
  import { onMount } from 'svelte';
  import { commands } from '$lib/bindings';
  import { sizeText } from '$lib/models';
  import { modelDownload as md, view } from '$lib/downloads.svelte';
  import DownloadRow from './DownloadRow.svelte';

  let model = $derived(md.model);
  let progress = $derived(md.progress);
  let item = $derived(view.items.find((i) => i.id === `model:${md.model?.id}`));
  let ask = $state('');
  let answer = $state('');
  let asking = $state(false);
  let error = $state('');

  async function tryModel() {
    asking = true;
    answer = '';
    error = '';
    const r = await commands.askModel(ask);
    if (r.status === 'ok') answer = r.data;
    else error = r.error;
    asking = false;
  }

  onMount(async () => {
    try {
      await md.load();
    } catch (e) {
      error = `Running outside the desktop app: ${String(e)}`;
    }
  });
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
    {:else if item && item.state !== 'done'}
      <div class="tb-dl-card"><DownloadRow {item} /></div>
    {:else}
      <div class="row"><button onclick={() => md.start()}>Download {model.name}</button></div>
      {#if progress?.error || md.loadError}<p role="alert">{progress?.error || md.loadError}</p>{/if}
    {/if}
  {/if}
</section>
{#if error}<p role="alert">{error}</p>{/if}

<style>
  section { display: flex; flex-direction: column; gap: 12px; }
  .row { display: flex; gap: 8px; flex-wrap: wrap; }
  .row input { flex: 1; min-width: 140px; }
</style>
