<script lang="ts">
  import { onMount } from 'svelte';
  import { commands, type JobLine } from '$lib/bindings';
  import { matchText, placeText, statusWord } from '$lib/jobs';
  import { searchSummary } from '$lib/boards';
  import EmptyState from '$lib/components/EmptyState.svelte';

  let jobs = $state<JobLine[]>([]);
  let loaded = $state(false);
  let error = $state('');

  let searching = $state(false);
  let summary = $state('');

  async function load() {
    try {
      const r = await commands.listJobs();
      if (r.status === 'ok') jobs = r.data;
      else error = r.error;
    } catch (e) {
      error = `Running outside the desktop app: ${String(e)}`;
    } finally {
      loaded = true;
    }
  }
  onMount(load);

  async function search() {
    searching = true;
    summary = '';
    const r = await commands.runSearch();
    if (r.status === 'ok') {
      summary = searchSummary(r.data);
      await load();
    } else error = r.error;
    searching = false;
  }
</script>

<span class="label">Jobs</span>
<p><button onclick={search} disabled={searching}>{searching ? 'Searching' : 'Search now'}</button></p>
{#if summary}<p role="status">{summary}</p>{/if}
{#if error}
  <p role="alert">{error}</p>
{:else if loaded && jobs.length === 0}
  <EmptyState title="Jobs CeeVee finds will appear here">
    <p>Add companies to watch in Settings, then choose Search now.</p>
  </EmptyState>
{:else if jobs.length > 0}
  <div class="plate">
    <table>
      <thead>
        <tr><th>Role</th><th>Company</th><th>Status</th><th>Match</th></tr>
      </thead>
      <tbody>
        {#each jobs as j (j.id)}
          <tr>
            <td><b>{j.title}</b><br />{placeText(j)}</td>
            <td>{j.company ?? 'Company not listed'}</td>
            <td><span class="dot" class:on={j.state === 'saved'}></span>{statusWord(j.state)}</td>
            <td class="n">{matchText(j.score)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}

<style>
  .plate { border: 1px solid var(--line); background: var(--surface); overflow-x: auto; }
  b { color: var(--ink); font-weight: 500; }
  .n { font-variant-numeric: tabular-nums; }
</style>
