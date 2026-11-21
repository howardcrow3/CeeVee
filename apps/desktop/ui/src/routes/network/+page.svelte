<script lang="ts">
  import { commands } from '$lib/bindings';
  import EmptyState from '$lib/components/EmptyState.svelte';

  let result = $state('');
  let busy = $state(false);

  // eslint-disable-next-line no-undef
  let files = $state<FileList | null>(null);

  async function pick() {
    const file = files?.[0];
    if (!file) return;
    busy = true;
    result = '';
    try {
      const r = await commands.importLinkedinConnections(await file.text());
      result = r.status === 'ok'
        ? `Added ${r.data.added} people and updated ${r.data.updated}.`
        : r.error;
    } catch (err) {
      result = `Could not read the file: ${String(err)}`;
    } finally {
      busy = false;
    }
  }
</script>

<span class="label">Network</span>
<EmptyState title="Who you know at each company">
  <p>Choose the Connections.csv file from your LinkedIn data export. CeeVee reads it on this computer and shows the people you know next to each job.</p>
  <div>
    <label class="button" class:disabled={busy}>
      Choose Connections.csv
      <input type="file" accept=".csv,text/csv" bind:files onchange={pick} disabled={busy} hidden />
    </label>
  </div>
  {#if result}<p role="status">{result}</p>{/if}
</EmptyState>

<style>
  .button { display: inline-block; }
  .disabled { opacity: 0.5; }
</style>
