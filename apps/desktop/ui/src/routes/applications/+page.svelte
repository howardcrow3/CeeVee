<script lang="ts">
  import { onMount } from 'svelte';
  import { commands, type ApplicationLine, type ProposalLine } from '$lib/bindings';
  import { applicationName, groupByStatus, statuses } from '$lib/applications';
  import { actionWords } from '$lib/approvals';
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

<span class="label">Applications</span>
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
    <div class="row">
      <button type="submit">Add application</button>
      <button type="button" class="ghost" onclick={() => (adding = false)}>Cancel</button>
    </div>
  </form>
{:else}
  <div class="row"><button onclick={() => (adding = true)}>Add application</button></div>
{/if}
{#if waiting.length > 0}
  <section>
    <h2>Waiting on you</h2>
    <ul>
      {#each waiting as p (p.id)}
        <li>
          <b>{actionWords(p.action)}</b>
          {#if p.reason}<p>{p.reason}</p>{/if}
          <div class="row">
            <button onclick={() => approve(p.id)}>Approve</button>
            <button class="ghost" onclick={() => skip(p.id)}>Skip</button>
          </div>
        </li>
      {/each}
    </ul>
  </section>
{/if}
{#each groupByStatus(apps) as g (g.status)}
  <section>
    <h2>{statuses.find((x) => x.value === g.status)?.label}</h2>
    <ul>
      {#each g.items as a (a.id)}
        <li>
          <b>{applicationName(a)}</b>
          <div class="row">
            <select aria-label="Status" value={a.status} onchange={(e) => move(a.id, e.currentTarget.value)}>
              {#each statuses as st (st.value)}<option value={st.value}>{st.label}</option>{/each}
            </select>
          </div>
        </li>
      {/each}
    </ul>
  </section>
{/each}
{#if waiting.length === 0 && apps.length === 0 && !error && !adding}
  <EmptyState title="No applications yet"><p>Applications you start appear here with their status. Press Add application to track one you applied to elsewhere.</p></EmptyState>
{/if}

<style>
  section { margin-top: 24px; max-width: 760px; }
  ul { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 16px; }
  li { border: 1px solid var(--line); background: var(--surface); padding: 16px; display: flex; flex-direction: column; gap: 8px; align-items: flex-start; }
  b { color: var(--ink); font-weight: 500; }
  p { margin: 0; }
  .row { display: flex; gap: 8px; }
  .add { margin-top: 16px; max-width: 760px; display: flex; flex-direction: column; gap: 12px; border: 1px solid var(--line); background: var(--surface); padding: 16px; }
  .add label { display: flex; flex-direction: column; gap: 4px; }
  input { font: inherit; background: var(--paper); color: var(--ink); border: 1px solid var(--border-control); padding: 8px 12px; }
  select { font: inherit; background: var(--paper); color: var(--ink); border: 1px solid var(--border-control); padding: 8px 12px; }
</style>
