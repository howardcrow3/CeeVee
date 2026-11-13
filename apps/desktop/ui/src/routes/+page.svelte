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

<span class="label">Today</span>
<h1>Your job search, on your own computer</h1>
<p>Nothing is waiting on you yet. Add your resume in Documents and set the kinds of jobs you want in Settings, and CeeVee starts searching.</p>
{#if info}<p>{describe(info)}</p>{/if}
{#if error}<p>Running outside the desktop app: {error}</p>{/if}
