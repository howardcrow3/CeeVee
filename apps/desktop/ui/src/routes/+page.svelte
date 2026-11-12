<script lang="ts">
  import { onMount } from 'svelte';
  import { appInfo, describe, type AppInfo } from '$lib/api';

  let info = $state<AppInfo | null>(null);
  let error = $state('');

  onMount(async () => {
    try {
      info = await appInfo();
    } catch (e) {
      error = String(e);
    }
  });
</script>

<main>
  <h1>CeeVee</h1>
  <p>Your job search, on your own computer.</p>
  {#if info}<p class="meta">{describe(info)}</p>{/if}
  {#if error}<p class="meta">Running outside the desktop app: {error}</p>{/if}
</main>

<style>
  main { font-family: system-ui, sans-serif; max-width: 40rem; margin: 4rem auto; padding: 0 1rem; }
  .meta { color: #666; font-size: 0.9rem; }
</style>
