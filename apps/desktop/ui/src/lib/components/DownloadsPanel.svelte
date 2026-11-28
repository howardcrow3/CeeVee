<script lang="ts">
  import { downloads, view } from '$lib/downloads.svelte';
  import { matches } from '$lib/turtlebox/tb-downloads.js';
  import DownloadRow from './DownloadRow.svelte';

  /** Toolbar button with a count badge while downloads run, and the Downloads popover. Hidden until there is something to show. */
  let open = $state(false);
  let query = $state('');
  let root: HTMLElement | undefined = $state();

  let ongoing = $derived(view.items.filter((i) => ['queued', 'running', 'paused'].includes(i.state) && matches(i, query)));
  let completed = $derived(
    view.items.filter((i) => !['queued', 'running', 'paused'].includes(i.state) && matches(i, query)).sort((a, b) => b.finishedAt - a.finishedAt)
  );
  // Close on its own when everything is cleared, so the button disappears with it.
  $effect(() => {
    if (view.items.length === 0) open = false;
  });

  function onkey(e: KeyboardEvent) {
    if (e.key === 'Escape') open = false;
  }
  function onpoint(e: MouseEvent) {
    if (open && root && !root.contains(e.target as Node)) open = false;
  }
</script>

<svelte:window onkeydown={onkey} onmousedown={onpoint} />

{#if view.items.length > 0}
  <div class="tb-dl-anchor" bind:this={root}>
    <button type="button" class="tb-dl-btn" aria-expanded={open} aria-haspopup="dialog" aria-label="Downloads, {view.active} active" title="Downloads" onclick={() => (open = !open)}>
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M8 2v8M4.5 7 8 10.5 11.5 7M3 13.5h10" /></svg>
      {#if view.active > 0}<span class="tb-dl-badge">{view.active}</span>{/if}
    </button>
    {#if open}
      <div class="tb-dl" role="dialog" aria-label="Downloads">
        <div class="tb-dl-head">
          <h2>Downloads</h2>
          <button type="button" class="tb-dl-act" aria-label="Close downloads" onclick={() => (open = false)}><svg viewBox="0 0 12 12" aria-hidden="true"><path d="M3 3l6 6M9 3l-6 6" /></svg></button>
        </div>
        <input class="tb-dl-filter" type="search" placeholder="Filter downloads..." aria-label="Filter downloads" bind:value={query} />
        {#if ongoing.length > 0}
          <div class="tb-dl-group">
            <div class="tb-dl-glabel">Ongoing</div>
            {#each ongoing as item (item.id)}<DownloadRow {item} />{/each}
          </div>
        {/if}
        {#if completed.length > 0}
          <div class="tb-dl-group">
            <div class="tb-dl-glabel">Completed <button type="button" class="tb-dl-link" onclick={() => downloads.clearCompleted()}>Clear all</button></div>
            {#each completed as item (item.id)}<DownloadRow {item} />{/each}
          </div>
        {/if}
        {#if ongoing.length === 0 && completed.length === 0}<div class="tb-dl-empty">Nothing matches.</div>{/if}
      </div>
    {/if}
  </div>
{/if}

<style>
  .tb-dl-anchor { position: fixed; top: 10px; right: 16px; z-index: 30; }
</style>
