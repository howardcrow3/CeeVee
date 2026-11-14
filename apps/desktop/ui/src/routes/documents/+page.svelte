<script lang="ts">
  import { onMount } from 'svelte';
  import { commands, type DocLine, type DocView } from '$lib/bindings';
  import { describeEdit, docKinds, isResume, kindLabel, starter } from '$lib/documents';
  import ResumeForm from '$lib/components/ResumeForm.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';

  let docs = $state<DocLine[]>([]);
  let open = $state<DocView | null>(null);
  let draft = $state('');
  let error = $state('');
  let message = $state('');
  let creating = $state(false);
  let newKind = $state('cover_letter');
  let newTitle = $state('');
  let loaded = $state(false);
  let asText = $state(false);
  let ask = $state('');
  let asking = $state(false);
  let held = $state<{ proposed: string; flagged: string[] } | null>(null);

  async function loadList() {
    try {
      const r = await commands.listDocuments();
      if (r.status === 'ok') docs = r.data;
      else error = r.error;
    } catch (e) {
      error = `Running outside the desktop app: ${String(e)}`;
    } finally {
      loaded = true;
    }
  }
  onMount(loadList);

  function show(d: DocView) {
    open = d;
    draft = d.content;
  }

  async function pick(id: number) {
    error = '';
    message = '';
    const r = await commands.getDocument(id);
    if (r.status === 'ok') show(r.data);
    else error = r.error;
  }

  async function create() {
    error = '';
    const r = await commands.createDocument(newKind, newTitle, starter(newKind));
    if (r.status === 'ok') {
      creating = false;
      newTitle = '';
      await loadList();
      await pick(r.data);
    } else error = r.error;
  }

  async function run(action: 'save' | 'undo' | 'redo') {
    if (!open) return;
    error = '';
    message = '';
    const r =
      action === 'save'
        ? await commands.saveDocument(open.id, draft)
        : action === 'undo'
          ? await commands.undoDocument(open.id)
          : await commands.redoDocument(open.id);
    if (r.status === 'ok') {
      show(r.data);
      message = action === 'save' ? 'Saved.' : action === 'undo' ? 'Went back one version.' : 'Went forward one version.';
    } else error = r.error;
  }

  async function keepHeld() {
    if (!held) return;
    draft = held.proposed;
    held = null;
    await run('save');
  }

  async function chat() {
    if (!open || !ask.trim()) return;
    error = '';
    message = '';
    asking = true;
    try {
      const r = await commands.chatEditDocument(open.id, ask.trim(), false);
      if (r.status === 'ok') {
        show(r.data.doc);
        message = describeEdit(r.data);
        held = r.data.applied ? null : { proposed: r.data.proposed, flagged: r.data.flagged };
        if (r.data.applied) ask = '';
      } else error = r.error;
    } finally {
      asking = false;
    }
  }
</script>

<span class="label">Documents</span>
{#if error}<p role="alert">{error}</p>{/if}
<div class="layout">
  <aside>
    {#if loaded && docs.length === 0 && !creating}
      <EmptyState title="No documents yet"><p>Your resume and cover letters are kept here, with every earlier version.</p></EmptyState>
    {/if}
    <ul>
      {#each docs as d (d.id)}
        <li>
          <button class="ghost item" class:cur={open?.id === d.id} onclick={() => pick(d.id)}>
            <span class="kind">{kindLabel(d.kind)}</span>{d.title}
          </button>
        </li>
      {/each}
    </ul>
    {#if creating}
      <div class="form">
        <select aria-label="Type" bind:value={newKind}>
          {#each docKinds as k (k.value)}<option value={k.value}>{k.label}</option>{/each}
        </select>
        <input aria-label="Title" placeholder="Title" bind:value={newTitle} />
        <div class="row">
          <button onclick={create}>Create</button>
          <button class="ghost" onclick={() => (creating = false)}>Cancel</button>
        </div>
      </div>
    {:else}
      <button onclick={() => (creating = true)}>New document</button>
    {/if}
  </aside>
  {#if open}
    <section>
      <h2>{open.title}</h2>
      <p class="note">
        {kindLabel(open.kind)}, {open.versions === 1 ? '1 version' : `${open.versions} versions`}.
      </p>
      {#if isResume(open.kind)}
        <div class="row">
          <button class="ghost" onclick={() => (asText = !asText)}>{asText ? 'Use the form' : 'Edit as text'}</button>
        </div>
      {/if}
      {#if isResume(open.kind) && !asText}
        <ResumeForm bind:value={draft} />
      {:else}
        <textarea aria-label="Document text" rows="18" bind:value={draft}></textarea>
      {/if}
      <div class="row">
        <button onclick={() => run('save')}>Save</button>
        <button class="ghost" onclick={() => run('undo')}>Undo</button>
        <button class="ghost" onclick={() => run('redo')}>Redo</button>
      </div>
      {#if message}<p role="status">{message}</p>{/if}
      <div class="chat">
        <span class="label">Ask for a change</span>
        <textarea aria-label="What should change" rows="3" placeholder="For example: make the opening shorter and warmer" bind:value={ask}></textarea>
        <div class="row">
          <button disabled={asking || !ask.trim()} onclick={() => chat()}>{asking ? 'Working…' : 'Make the change'}</button>
          {#if held}<button class="ghost" onclick={keepHeld}>Save it anyway</button>{/if}
        </div>
        {#if held}<pre class="held">{held.proposed}</pre>{/if}
      </div>
    </section>
  {/if}
</div>

<style>
  .layout { display: grid; grid-template-columns: 240px 1fr; gap: 24px; margin-top: 24px; }
  @media (max-width: 700px) { .layout { grid-template-columns: 1fr; } }
  aside, section { display: flex; flex-direction: column; gap: 16px; min-width: 0; }
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 8px; }
  .item { width: 100%; text-align: left; text-transform: none; letter-spacing: 0; font-size: 14px; display: flex; flex-direction: column; gap: 4px; }
  .item.cur { border-color: var(--accent); }
  .kind { font-size: 11px; letter-spacing: 0.3em; text-transform: uppercase; color: var(--body); }
  .row { display: flex; gap: 8px; flex-wrap: wrap; }
  .form { display: flex; flex-direction: column; gap: 8px; }
  textarea { font: 400 14px/1.6 var(--font-body, inherit); background: var(--paper); color: var(--ink); border: 1px solid var(--border-control); padding: 16px; width: 100%; resize: vertical; }
  select { font: inherit; background: var(--paper); color: var(--ink); border: 1px solid var(--border-control); padding: 12px 16px; }
  p { margin: 0; }
  .chat { display: flex; flex-direction: column; gap: 8px; border-top: 1px solid var(--border-control); padding-top: 16px; }
  .held { white-space: pre-wrap; font: 400 13px/1.5 var(--font-body, inherit); color: var(--body); border: 1px solid var(--border-control); padding: 12px; margin: 0; }
</style>
