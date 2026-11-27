<script lang="ts">
  import { onMount } from 'svelte';
  import { commands, type JobDraft, type JobLine, type KnownLine } from '$lib/bindings';
  import { byMatch, filterJobs, draftLabel, draftNote, knownText, matchText, placeText, prepareSummary, selectedText, statusWord, stepId, toggleAllIds, toggleId } from '$lib/jobs';
  import { searchSummary } from '$lib/boards';
  import ChatLayout from '$lib/components/ChatLayout.svelte';
  import { jobFocus } from '$lib/chat';
  import EmptyState from '$lib/components/EmptyState.svelte';

  let jobs = $state<JobLine[]>([]);
  let loaded = $state(false);
  let stateFilter = $state<'all' | 'new' | 'saved'>('all');
  let text = $state('');
  let openId = $state<number | null>(null);
  const shown = $derived(byMatch(filterJobs(jobs, { state: stateFilter, text })));
  const open = $derived(jobs.find((j) => j.id === openId) ?? null);
  const chips: { value: 'all' | 'new' | 'saved'; label: string }[] = [
    { value: 'all', label: 'All' },
    { value: 'new', label: 'New' },
    { value: 'saved', label: 'Saved' }
  ];
  let picked = $state<Set<number>>(new Set());
  const allPicked = $derived(shown.length > 0 && shown.every((j) => picked.has(j.id)));

  function onRowKey(e: KeyboardEvent, id: number) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      openId = id;
    } else if (e.key === 'ArrowDown' || e.key === 'j' || e.key === 'ArrowUp' || e.key === 'k') {
      e.preventDefault();
      const next = stepId(shown.map((j) => j.id), id, e.key === 'ArrowDown' || e.key === 'j' ? 1 : -1);
      if (next != null) (document.querySelector(`[data-job="${next}"]`) as HTMLElement | null)?.focus();
    } else if (e.key === 'x') {
      e.preventDefault();
      picked = toggleId(picked, id);
    }
  }

  async function dismissPicked() {
    applyError = '';
    for (const id of picked) {
      const r = await commands.dismissJob(id);
      if (r.status !== 'ok') {
        applyError = r.error;
        break;
      }
    }
    picked = new Set();
    openId = null;
    await load();
  }
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
<ChatLayout focus={open ? jobFocus(open) : ''}>

<header class="head">
  <span class="label">Jobs</span>
  <button onclick={search} disabled={searching}>{searching ? 'Searching' : 'Search now'}</button>
</header>
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
  <div class="bar">
    <div class="seg" role="group" aria-label="Status">
      {#each chips as c (c.value)}
        <button class:on={stateFilter === c.value} aria-pressed={stateFilter === c.value} onclick={() => (stateFilter = c.value)}>{c.label}</button>
      {/each}
    </div>
    <input class="find" aria-label="Filter jobs" placeholder="Filter by title, company or place" bind:value={text} />
    <span class="count note">{shown.length} of {jobs.length}</span>
  </div>
  {#if picked.size > 0}
    <div class="bulk" role="region" aria-label="Selected jobs">
      <b>{selectedText(picked.size)}</b>
      <button class="ghost" onclick={dismissPicked}>Dismiss selected</button>
      <button class="ghost" onclick={() => (picked = new Set())}>Clear</button>
    </div>
  {/if}
  <div class="split" class:with-detail={open}>
    <div class="plate">
      <table>
        <thead>
          <tr><th class="pick"><input type="checkbox" aria-label="Select all shown" checked={allPicked} onchange={() => (picked = toggleAllIds(picked, shown.map((j) => j.id)))} /></th><th>Role</th><th>Company</th><th>Place</th><th>Status</th><th class="n">Match</th></tr>
        </thead>
        <tbody>
          {#each shown as j (j.id)}
            <tr class="jr" class:sel={openId === j.id} data-job={j.id} tabindex="0" onclick={() => (openId = j.id)} onkeydown={(e) => onRowKey(e, j.id)}>
              <td class="pick" onclick={(e) => e.stopPropagation()}><input type="checkbox" aria-label={`Select ${j.title}`} checked={picked.has(j.id)} onchange={() => (picked = toggleId(picked, j.id))} /></td>
              <td class="t"><b>{j.title}</b></td>
              <td class="t">{j.company ?? 'Company not listed'}</td>
              <td class="t">{placeText(j)}</td>
              <td><span class="dot" class:on={j.state === 'saved'}></span>{statusWord(j.state)}</td>
              <td class="n">{matchText(j.score)}</td>
            </tr>
          {:else}
            <tr><td colspan="6" class="none">No jobs match this filter.</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
    {#if open}
      <aside class="detail" aria-label="Job detail">
        <div class="dhead">
          <b>{open.title}</b>
          <button class="ghost" aria-label="Close detail" onclick={() => (openId = null)}>Close</button>
        </div>
        <div class="note">{open.company ?? 'Company not listed'} · {placeText(open)}</div>
        <div class="note"><span class="dot" class:on={open.state === 'saved'}></span>{statusWord(open.state)} · Match {matchText(open.score)}</div>
        {#if open.known > 0}
          <button class="link" onclick={() => showPeople(open.id)}>{knownText(open.known)}</button>
          {#if who && who.job === open.id}
            <ul class="people">{#each who.people as p (p.name)}<li>{p.name}{p.title ? `, ${p.title}` : ''}</li>{/each}</ul>
          {/if}
        {/if}
        <div class="acts">
          <button class="ghost" onclick={() => openPosting(open.url)}>Open posting</button>
          <button class="ghost" disabled={drafting !== null} onclick={() => writeDraft(open.id, 'cover_letter')}>{drafting === `${open.id}:cover_letter` ? 'Writing' : 'Write cover letter'}</button>
          <button class="ghost" disabled={drafting !== null} onclick={() => writeDraft(open.id, 'tailored_resume')}>{drafting === `${open.id}:tailored_resume` ? 'Writing' : 'Tailor resume'}</button>
          <button disabled={preparing !== null} onclick={() => prepare(open.id)}>{preparing === open.id ? 'Opening' : 'Prepare application'}</button>
          <button class="ghost" onclick={() => { const id = open.id; openId = null; dismiss(id); }}>Dismiss</button>
        </div>
      </aside>
    {/if}
  </div>
{/if}
</ChatLayout>

<style>
  .head { display: flex; align-items: center; justify-content: space-between; }
  .bar { display: flex; align-items: center; gap: 8px; }
  .find { flex: 1; max-width: 320px; }
  .seg { display: inline-flex; border: 1px solid var(--line); border-radius: var(--r-sm); overflow: hidden; }
  .seg button { height: 26px; padding: 0 10px; border: 0; border-radius: 0; background: transparent; color: var(--body); font-size: 12.5px; }
  .seg button.on { background: var(--selected); color: var(--ink); }
  .seg button:hover:not(:disabled):not(.on) { background: var(--hover); }
  .count { margin-left: auto; }
  .split { display: grid; grid-template-columns: 1fr; gap: 12px; align-items: start; }
  .split.with-detail { grid-template-columns: minmax(0, 1fr) 300px; }
  .plate { border: 1px solid var(--line); border-radius: var(--r-md); background: var(--surface); overflow-x: auto; }
  .plate td, .plate th { padding: 0 12px; height: 30px; vertical-align: middle; }
  .plate th { height: 26px; }
  .plate tr:last-child td { border-bottom: 0; }
  .t { max-width: 240px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .jr { cursor: pointer; }
  .jr:hover { background: var(--hover); }
  .jr.sel { background: var(--selected); }
  .jr:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  .none { color: var(--muted); }
  .pick { width: 28px; padding-right: 0; }
  .bulk { display: flex; align-items: center; gap: 8px; padding: 4px 12px; border: 1px solid var(--line); border-radius: var(--r-md); background: var(--selected); }
  b { color: var(--ink); font-weight: 500; }
  .n { font-variant-numeric: tabular-nums; text-align: right; }
  .detail { border: 1px solid var(--line); border-radius: var(--r-md); background: var(--surface); padding: 12px; display: flex; flex-direction: column; gap: 8px; }
  .dhead { display: flex; align-items: flex-start; justify-content: space-between; gap: 8px; }
  .acts { display: flex; flex-direction: column; gap: 6px; align-items: stretch; margin-top: 4px; }
  .draft { display: flex; flex-direction: column; gap: 12px; max-width: 760px; margin: 16px 0; }
  .draft textarea { font: inherit; background: var(--paper); color: var(--ink); border: 1px solid var(--border-control); border-radius: var(--r-sm); padding: 8px 12px; }
  .row { display: flex; gap: 8px; }
  .people { margin: 4px 0 0; padding-left: 16px; }
</style>
