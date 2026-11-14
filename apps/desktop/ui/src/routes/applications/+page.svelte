<script lang="ts">
  import { onMount } from 'svelte';
  import { commands, type ProposalLine } from '$lib/bindings';
  import { actionWords } from '$lib/approvals';
  import EmptyState from '$lib/components/EmptyState.svelte';

  let waiting = $state<ProposalLine[]>([]);
  let error = $state('');

  async function load() {
    try {
      const r = await commands.listProposals();
      if (r.status === 'ok') waiting = r.data;
      else error = r.error;
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
</script>

<span class="label">Applications</span>
{#if error}<p role="alert">{error}</p>{/if}
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
{:else if !error}
  <EmptyState title="No applications yet"><p>Applications you start appear here with their status.</p></EmptyState>
{/if}

<style>
  section { margin-top: 24px; max-width: 760px; }
  ul { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 16px; }
  li { border: 1px solid var(--line); background: var(--surface); padding: 16px; display: flex; flex-direction: column; gap: 8px; align-items: flex-start; }
  b { color: var(--ink); font-weight: 500; }
  p { margin: 0; }
  .row { display: flex; gap: 8px; }
</style>
