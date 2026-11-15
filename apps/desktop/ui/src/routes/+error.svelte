<script lang="ts">
  import { page } from '$app/state';
  import { resolve } from '$app/paths';
</script>

<span class="label">Something went wrong</span>
<h1>This page did not load</h1>
<p>
  Your jobs and documents are safe. Go back to <a href={resolve('/')}>Today</a> and try again. If it keeps
  happening, restart CeeVee, and send the log files from Settings to support.
</p>
{#if page.error?.message}<p class="note">{page.error.message}</p>{/if}
