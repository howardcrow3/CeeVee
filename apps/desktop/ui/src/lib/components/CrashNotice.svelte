<script lang="ts">
  import { onMount } from 'svelte';
  import { commands } from '$lib/bindings';

  let summary = $state<string | null>(null);

  async function dismiss() {
    summary = null;
    await commands.dismissCrash();
  }
  async function openLogs() {
    await commands.openLogFolder();
  }

  onMount(async () => {
    try {
      const r = await commands.diagnostics();
      if (r.status === 'ok') summary = r.data.last_crash;
    } catch {
      // Running outside the desktop app: nothing to show.
    }
  });
</script>

{#if summary}
  <div class="banner" role="alert">
    <span>CeeVee closed unexpectedly last time ({summary}). Your jobs and documents are safe. If it keeps happening, send the files in the log folder to support.</span>
    <span class="row">
      <button class="ghost" onclick={openLogs}>Open log folder</button>
      <button class="ghost" onclick={dismiss}>Dismiss</button>
    </span>
  </div>
{/if}

<style>
  .banner { display: flex; justify-content: space-between; gap: 16px; align-items: baseline; border: 1px solid var(--danger); background: var(--surface); padding: 12px 16px; }
  .row { display: flex; gap: 8px; flex-shrink: 0; }
</style>
