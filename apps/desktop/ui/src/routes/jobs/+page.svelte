<script lang="ts">
  import { onMount } from 'svelte';
  import { commands, type JobDraft, type JobLine, type KnownLine } from '$lib/bindings';
  import { byMatch, draftLabel, draftNote, knownText, matchText, placeText, prepareSummary, statusWord } from '$lib/jobs';
  import { searchSummary } from '$lib/boards';
  import EmptyState from '$lib/components/EmptyState.svelte';

  let jobs = $state<JobLine[]>([]);
  let loaded = $state(false);
  let error = $state('');

  let searching = $state(false);
  let summary = $state('');
  let preparing = $state<number | null>(null);
  let applyError = $state('');

  async function prepare(id: number) {
    preparing = id;
    summary = '';
    applyError = '';
    try {
      const r = await commands.prepareApplication(id);
      if (r.status === 'ok') summary = prepareSummary(r.data);
      else applyError = r.error;
    } finally {
      preparing = null;
    }
  }

  let who = $state<{ job: number; people: KnownLine[] } | null>(null);
  async function showPeople(id: number) {
    const r = await commands.peopleAtJob(id);
    if (r.status === 'ok') who = { job: id, people: r.data };
    else applyError = r.error;
  }

  let draft = $state<{ job: number; data: JobDraft } | null>(null);
  let drafting = $state<string | null>(null);
  let draftMessage = $state('');

  async function writeDraft(job: number, kind: string) {
    drafting = `${job}:${kind}`;
    draftMessage = '';
    applyError = '';
    try {
      const r = await commands.draftForJob(job, kind, '');
      if (r.status === 'ok') draft = { job, data: r.data };
      else applyError = r.error;
    } finally {
      drafting = null;
    }
  }

  async function saveDraft() {
    if (!draft) return;
    const r = await commands.saveJobDraft(draft.job, draft.data.kind, draft.data.content);
    if (r.status === 'ok') {
      draftMessage = `Saved the ${draftLabel(draft.data.kind)} in Documents.`;
      draft = null;
    } else applyError = r.error;
  }

  async function dismiss(id: number) {
    const r = await commands.dismissJob(id);
    if (r.status === 'ok') await load();
    else applyError = r.error;
  }

  async function openPosting(url: string) {
    const r = await commands.openPosting(url);
    if (r.status !== 'ok') applyError = r.error;
  }

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
{#if applyError}<p role="alert">{applyError}</p>{/if}
{#if draftMessage}<p role="status">{draftMessage}</p>{/if}
{#if draft}
  <section class="draft">
    <h2>Your {draftLabel(draft.data.kind)}</h2>
    <p class="note">{draftNote(draft.data.flagged)}</p>
    <textarea aria-label="Draft" rows="14" bind:value={draft.data.content}></textarea>
    <div class="row">
      <button onclick={saveDraft}>Save to Documents</button>
      <button class="ghost" onclick={() => (draft = null)}>Throw away</button>
    </div>
  </section>
{/if}
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
        <tr><th>Role</th><th>Company</th><th>Status</th><th>Match</th><th></th></tr>
      </thead>
      <tbody>
        {#each byMatch(jobs) as j (j.id)}
          <tr>
            <td><b>{j.title}</b><br />{placeText(j)}</td>
            <td>{j.company ?? 'Company not listed'}{#if j.known > 0}<br /><button class="link" onclick={() => showPeople(j.id)}>{knownText(j.known)}</button>{/if}
              {#if who && who.job === j.id}
                <ul class="people">{#each who.people as p (p.name)}<li>{p.name}{p.title ? `, ${p.title}` : ''}</li>{/each}</ul>
              {/if}</td>
            <td><span class="dot" class:on={j.state === 'saved'}></span>{statusWord(j.state)}</td>
            <td class="n">{matchText(j.score)}</td>
            <td class="actions"><button class="ghost" onclick={() => openPosting(j.url)}>Open posting</button> <button class="ghost" onclick={() => dismiss(j.id)}>Dismiss</button> <button class="ghost" disabled={drafting !== null} onclick={() => writeDraft(j.id, 'cover_letter')}>{drafting === `${j.id}:cover_letter` ? 'Writing' : 'Write cover letter'}</button> <button class="ghost" disabled={drafting !== null} onclick={() => writeDraft(j.id, 'tailored_resume')}>{drafting === `${j.id}:tailored_resume` ? 'Writing' : 'Tailor resume'}</button> <button class="ghost" disabled={preparing !== null} onclick={() => prepare(j.id)}>{preparing === j.id ? 'Opening' : 'Prepare application'}</button></td>
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
  .actions { white-space: normal; }
  .draft { display: flex; flex-direction: column; gap: 12px; max-width: 760px; margin: 16px 0; }
  .draft textarea { font: inherit; background: var(--paper); color: var(--ink); border: 1px solid var(--border-control); padding: 12px 16px; }
  .row { display: flex; gap: 8px; }
  .link { background: none; border: none; padding: 0; color: var(--ink); text-decoration: underline; cursor: pointer; font: inherit; }
  .people { margin: 4px 0 0; padding-left: 16px; }
</style>
