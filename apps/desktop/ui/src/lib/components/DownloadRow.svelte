<script lang="ts">
  import { percent, statusLine } from '$lib/turtlebox/tb-downloads.js';
  import type { DownloadItem } from '$lib/turtlebox/tb-downloads.js';

  /** One download: name, progress, status line and its actions. Used in the panel and in inline cards. */
  let { item }: { item: DownloadItem } = $props();
  let a = $derived(item.actions ?? {});
  let finished = $derived(item.state === 'done' || item.state === 'canceled');
</script>

<div class="tb-dl-item" data-state={item.state}>
  <div class="tb-dl-name">{item.name}</div>
  <div class="tb-dl-acts">
    {#if item.state === 'running' && a.pause}
      <button type="button" class="tb-dl-act" aria-label="Pause {item.name}" title="Pause" onclick={a.pause}><svg viewBox="0 0 12 12" aria-hidden="true"><path d="M4 2.5v7M8 2.5v7" /></svg></button>
    {/if}
    {#if item.state === 'paused' && a.resume}
      <button type="button" class="tb-dl-act" aria-label="Resume {item.name}" title="Resume" onclick={a.resume}><svg viewBox="0 0 12 12" aria-hidden="true"><path d="M4 2.5 9.5 6 4 9.5z" /></svg></button>
    {/if}
    {#if (item.state === 'running' || item.state === 'queued' || item.state === 'paused') && a.cancel}
      <button type="button" class="tb-dl-act" aria-label="Cancel {item.name}" title="Cancel" onclick={a.cancel}><svg viewBox="0 0 12 12" aria-hidden="true"><path d="M3 3l6 6M9 3l-6 6" /></svg></button>
    {/if}
    {#if (item.state === 'failed' || item.state === 'canceled') && a.retry}
      <button type="button" class="tb-dl-act" aria-label="Retry {item.name}" title="Retry" onclick={a.retry}><svg viewBox="0 0 12 12" aria-hidden="true"><path d="M10 6a4 4 0 1 1-1.2-2.8M10 2v2.5H7.5" /></svg></button>
    {/if}
  </div>
  {#if !finished}
    <div class="tb-progress" role="progressbar" aria-label="{item.name} progress" aria-valuemin="0" aria-valuemax="100" aria-valuenow={percent(item.done, item.total)} style="--v:{item.total > 0 ? item.done / item.total : 0}"><i></i></div>
  {/if}
  <div class="tb-dl-meta" role="status">{statusLine(item)}</div>
</div>
