<script lang="ts">
  import { onMount } from 'svelte';
  import { commands, type PersonLine } from '$lib/bindings';
  import { importSummary, tiers, workLine } from '$lib/people';
  import EmptyState from '$lib/components/EmptyState.svelte';

  let people = $state<PersonLine[]>([]);
  let query = $state('');
  let tier = $state('');
  let result = $state('');
  let error = $state('');
  let busy = $state(false);
  let loaded = $state(false);

  // eslint-disable-next-line no-undef
  let linkedinFiles = $state<FileList | null>(null);
  // eslint-disable-next-line no-undef
  let contactFiles = $state<FileList | null>(null);

  async function load() {
    try {
      const r = await commands.listPeople(query, tier);
      if (r.status === 'ok') {
        people = r.data;
        error = '';
      } else error = r.error;
    } catch (e) {
      error = `Running outside the desktop app: ${String(e)}`;
    } finally {
      loaded = true;
    }
  }
  onMount(load);

  async function importLinkedin() {
    const file = linkedinFiles?.[0];
    if (!file) return;
    busy = true;
    result = '';
    try {
      const r = await commands.importLinkedinConnections(await file.text());
      result = r.status === 'ok' ? `Added ${r.data.added} people and updated ${r.data.updated}.` : r.error;
      await load();
    } catch (err) {
      result = `Could not read the file: ${String(err)}`;
    } finally {
      busy = false;
    }
  }

  async function importContacts() {
    const file = contactFiles?.[0];
    if (!file) return;
    busy = true;
    result = '';
    try {
      const r = await commands.importContacts(file.name, await file.text());
      result = r.status === 'ok' ? importSummary(r.data) : r.error;
      await load();
    } catch (err) {
      result = `Could not read the file: ${String(err)}`;
    } finally {
      busy = false;
    }
  }

  async function setTier(id: number, value: string) {
    const r = await commands.setPersonTier(id, value);
    if (r.status !== 'ok') error = r.error;
    await load();
  }
</script>

<span class="label">Network</span>
{#if error}<p role="alert">{error}</p>{/if}
<div class="row">
  <label class="button" class:disabled={busy}>
    Import contacts
    <input type="file" accept=".vcf,.csv,text/vcard,text/csv" bind:files={contactFiles} onchange={importContacts} disabled={busy} hidden />
  </label>
  <label class="button ghost" class:disabled={busy}>
    Import LinkedIn Connections.csv
    <input type="file" accept=".csv,text/csv" bind:files={linkedinFiles} onchange={importLinkedin} disabled={busy} hidden />
  </label>
</div>
{#if result}<p role="status">{result}</p>{/if}

{#if people.length === 0 && !query && !tier && loaded && !error}
  <EmptyState title="Who you know">
    <p>Import a contacts file (.vcf or .csv from Google, Outlook or your phone) or your LinkedIn Connections.csv. CeeVee reads it on this computer and shows the people you know next to each job.</p>
  </EmptyState>
{:else}
  <div class="row filters">
    <input type="search" placeholder="Search people" aria-label="Search people" bind:value={query} oninput={load} />
    <select aria-label="Closeness" bind:value={tier} onchange={load}>
      <option value="">Everyone</option>
      {#each tiers as t (t.value)}<option value={t.value}>{t.label}</option>{/each}
    </select>
  </div>
  <ul>
    {#each people as p (p.id)}
      <li>
        <div class="who">
          <b>{p.name}</b>
          <span class="muted">{workLine(p.title, p.company)}</span>
        </div>
        <select aria-label="Closeness of {p.name}" value={p.tier} onchange={(e) => setTier(p.id, e.currentTarget.value)}>
          {#each tiers as t (t.value)}<option value={t.value}>{t.label}</option>{/each}
        </select>
      </li>
    {:else}
      <li>No one matches.</li>
    {/each}
  </ul>
{/if}

<style>
  .row { display: flex; gap: 8px; margin-top: 16px; align-items: center; }
  .filters input { flex: 1; max-width: 360px; }
  ul { list-style: none; padding: 0; margin: 16px 0 0; max-width: 760px; border: 1px solid var(--line); background: var(--surface); }
  li { display: flex; justify-content: space-between; align-items: center; gap: 12px; padding: 8px 12px; border-bottom: 1px solid var(--line); }
  li:last-child { border-bottom: none; }
  .who { display: flex; flex-direction: column; min-width: 0; }
  b { color: var(--ink); font-weight: 500; }
  .muted { color: var(--ink-muted, inherit); opacity: 0.8; }
  .button { display: inline-block; }
  .disabled { opacity: 0.5; }
  input, select { font: inherit; background: var(--paper); color: var(--ink); border: 1px solid var(--border-control); padding: 6px 10px; }
</style>
