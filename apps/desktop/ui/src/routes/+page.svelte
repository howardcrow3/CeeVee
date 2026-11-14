<script lang="ts">
  import { onMount } from 'svelte';
  import { appInfo, describe, type AppInfo } from '$lib/api';
  import { resolve } from '$app/paths';
  import { commands } from '$lib/bindings';

  let info = $state<AppInfo | null>(null);
  let error = $state('');
  let waiting = $state(0);

  onMount(async () => {
    try {
      info = await appInfo();
      const n = await commands.pendingApprovalCount();
      if (n.status === 'ok') waiting = n.data;
    } catch (e) {
      error = String(e);
    }
  });
</script>

<span class="label">Today</span>
<h1>Your job search, on your own computer</h1>
{#if waiting > 0}
  <p><span class="dot on"></span>{waiting} {waiting === 1 ? 'item is' : 'items are'} waiting for your approval. <a href="{resolve('/applications')}">See them</a></p>
{:else}
  <p>Nothing is waiting on you yet. Add your resume in Documents and set the kinds of jobs you want in Settings, and CeeVee starts searching.</p>
{/if}
{#if info}<p>{describe(info)}</p>{/if}
{#if error}<p>Running outside the desktop app: {error}</p>{/if}
