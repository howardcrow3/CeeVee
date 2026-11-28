<script lang="ts">
  import { onMount } from 'svelte';
  import { modelDownload as md, view } from '$lib/downloads.svelte';
  import { setupView, sizeText } from '$lib/models';
  import DownloadRow from './DownloadRow.svelte';

  /** Shown on a fresh install: offers the local AI download, with progress. Hidden once it is installed. */
  let { onready = () => {} }: { onready?: () => void } = $props();

  let item = $derived(view.items.find((i) => i.id === `model:${md.model?.id}`));
  let v = $derived(setupView(md.model, md.progress, md.loadError));

  $effect(() => {
    if (md.model?.installed) onready();
  });
  onMount(() => md.load());
</script>

{#if v !== 'ready' && v !== 'checking'}
  <section class="card" aria-label="Set up the local AI">
    {#if v === 'downloading' && item}
      <strong>Downloading the local AI</strong>
      <div class="tb-dl-card"><DownloadRow {item} /></div>
      <span class="note">You can keep using CeeVee meanwhile. Progress is also under the download button at the top right.</span>
    {:else}
      <strong>CeeVee needs its local AI to chat</strong>
      {#if md.model}
        <span class="note">{md.model.name} ({sizeText(md.model.size_mb)}) runs on this computer, and nothing you write leaves it. Download it once and chat works.</span>
        <div><button onclick={() => md.start()}>{v === 'failed' ? 'Try the download again' : `Download ${md.model.name}`}</button></div>
      {/if}
      {#if md.loadError || md.progress?.error}<span role="alert">{md.loadError || md.progress?.error}</span>{/if}
    {/if}
  </section>
{/if}

<style>
  .card { display: flex; flex-direction: column; gap: 8px; border: 1px solid var(--line); background: var(--surface); border-radius: var(--r-md); padding: 12px 16px; max-width: 720px; width: 100%; margin: 0 auto; box-sizing: border-box; }
</style>
