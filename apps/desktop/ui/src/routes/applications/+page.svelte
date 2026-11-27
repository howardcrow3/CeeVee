<script lang="ts">
  import { onMount } from 'svelte';
  import { commands, type ApplicationLine, type ProposalLine } from '$lib/bindings';
  import { applicationName, boardColumns, groupByStatus, statuses } from '$lib/applications';
  import { actionWords } from '$lib/approvals';
  import ChatLayout from '$lib/components/ChatLayout.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';

  let waiting = $state<ProposalLine[]>([]);
  let apps = $state<ApplicationLine[]>([]);
  let error = $state('');

  async function load() {
    try {
      const r = await commands.listProposals();
      if (r.status === 'ok') waiting = r.data;
      else error = r.error;
      const a = await commands.listApplications();
      if (a.status === 'ok') apps = a.data;
      else error = a.error;
    } catch (e) {
      error = `Running outside the desktop app: ${String(e)}`;
    }
  }
  onMount(load);

  async function approve(id: number) {
    error = '';
    const r = await commands.approveProposal(id);
    if (r.status === 'ok') await load();
    else error = r.error;
  }

  async function skip(id: number) {
    const r = await commands.skipProposal(id);
    if (r.status === 'ok') await load();
    else error = r.error;
  }

  let view = $state<'board' | 'table'>('board');
  let expanded = $state<Set<string>>(new Set());
  const columns = $derived(boardColumns(apps, 5, expanded));
  function expand(key: string) {
    expanded = new Set([...expanded, key]);
  }

  let adding = $state(false);
  let company = $state('');
  let title = $state('');
  let status = $state('sent');

  async function add() {
    error = '';
    const r = await commands.addApplication(company, title, status);
    if (r.status === 'ok') {
      company = '';
      title = '';
      adding = false;
      await load();
    } else error = r.error;
  }

  async function move(id: number, status: string) {
    error = '';
    const r = await commands.setApplicationStatus(id, status);
    await load();
    if (r.status !== 'ok') error = r.error;
  }
</script>
<ChatLayout>

<header class="head">
  <span class="label">Applications</span>
  <div class="tools">
    <div class="seg" role="group" aria-label="View">
      <button class:on={view === 'board'} aria-pressed={view === 'board'} onclick={() => (view = 'board')}>Board</button>
      <button class:on={view === 'table'} aria-pressed={view === 'table'} onclick={() => (view = 'table')}>Table</button>
    </div>
    <button onclick={() => (adding = !adding)}>{adding ? 'Cancel' : 'Add application'}</button>
  </div>
</header>
{#if error}<p role="alert">{error}</p>{/if}
{#if adding}
  <form class="add" onsubmit={(e) => { e.preventDefault(); add(); }}>
    <label>Company <input bind:value={company} required /></label>
    <label>Job title <input bind:value={title} required /></label>
    <label>
      Status
      <select bind:value={status}>
        {#each statuses as st (st.value)}<option value={st.value}>{st.label}</option>{/each}
      </select>
    </label>
    <div class="row"><button type="submit">Add application</button></div>
  </form>
{/if}
{#if waiting.length > 0}
  <section class="waiting" aria-label="Waiting on you">
    <h2>Waiting on you <span class="note">{waiting.length}</span></h2>
    <ul class="cards">
      {#each waiting as p (p.id)}
        <li>
          <b>{actionWords(p.action)}</b>
          {#if p.reason}<span class="note">{p.reason}</span>{/if}
          <div class="row">
            <button onclick={() => approve(p.id)}>Approve</button>
            <button class="ghost" onclick={() => skip(p.id)}>Skip</button>
          </div>
        </li>
      {/each}
    </ul>
  </section>
{/if}
{#if apps.length > 0 && view === 'board'}
  <div class="board">
    {#each columns as c (c.key)}
      <section class="col" aria-label={c.label}>
        <h2>{c.label} <span class="note">{c.total}</span></h2>
        <ul class="cards">
          {#each c.cards as a (a.id)}
            <li>
              <b>{a.title}</b>
              <span class="note">{a.company ?? 'Company not listed'}</span>
              <select aria-label={`Status of ${applicationName(a)}`} value={a.status} onchange={(e) => move(a.id, e.currentTarget.value)}>
                {#each statuses as st (st.value)}<option value={st.value}>{st.label}</option>{/each}
              </select>
            </li>
          {/each}
        </ul>
        {#if c.more > 0}<button class="ghost more" onclick={() => expand(c.key)}>+{c.more} more</button>{/if}
      </section>
    {/each}
  </div>
{:else if apps.length > 0}
  {#each groupByStatus(apps) as g (g.status)}
    <section class="group">
      <h2>{statuses.find((x) => x.value === g.status)?.label} <span class="note">{g.items.length}</span></h2>
      <ul class="rows">
        {#each g.items as a (a.id)}
          <li>
            <b>{applicationName(a)}</b>
            <select aria-label="Status" value={a.status} onchange={(e) => move(a.id, e.currentTarget.value)}>
              {#each statuses as st (st.value)}<option value={st.value}>{st.label}</option>{/each}
            </select>
          </li>
        {/each}
      </ul>
    </section>
  {/each}
{/if}
{#if waiting.length === 0 && apps.length === 0 && !error && !adding}
  <EmptyState title="No applications yet"><p>Applications you start appear here with their status. Press Add application to track one you applied to elsewhere.</p></EmptyState>
{/if}
</ChatLayout>

<style>
  .head { display: flex; align-items: center; justify-content: space-between; }
  .tools { display: flex; gap: 8px; align-items: center; }
  .seg { display: inline-flex; border: 1px solid var(--line); border-radius: var(--r-sm); overflow: hidden; }
  .seg button { height: 26px; padding: 0 10px; border: 0; border-radius: 0; background: transparent; color: var(--body); font-size: 12.5px; }
  .seg button.on { background: var(--selected); color: var(--ink); }
  .seg button:hover:not(:disabled):not(.on) { background: var(--hover); }
  h2 { font-size: 13px; margin: 0 0 8px; display: flex; gap: 6px; align-items: baseline; }
  .board { display: grid; grid-template-columns: repeat(6, minmax(150px, 1fr)); gap: 8px; align-items: start; overflow-x: auto; }
  .col { background: var(--sidebar); border: 1px solid var(--line); border-radius: var(--r-md); padding: 8px; min-width: 0; }
  .cards, .rows { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 6px; }
  .cards li { border: 1px solid var(--line); background: var(--surface); border-radius: var(--r-sm); padding: 8px; display: flex; flex-direction: column; gap: 4px; }
  .cards select { width: 100%; }
  .more { width: 100%; margin-top: 6px; }
  .waiting { border: 1px solid var(--line); border-radius: var(--r-md); padding: 8px; background: var(--surface); }
  .waiting .cards { flex-direction: row; flex-wrap: wrap; }
  .waiting .cards li { min-width: 220px; flex: 1; }
  .group { max-width: 760px; }
  .rows li { display: flex; justify-content: space-between; align-items: center; gap: 12px; border-bottom: 1px solid var(--line); min-height: 30px; }
  b { color: var(--ink); font-weight: 500; }
  .row { display: flex; gap: 8px; }
  .add { max-width: 480px; display: flex; flex-direction: column; gap: 8px; border: 1px solid var(--line); border-radius: var(--r-md); background: var(--surface); padding: 12px; }
  .add label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--muted); }
</style>
