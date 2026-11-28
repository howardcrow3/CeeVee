<script lang="ts">
  import { openSettings } from '$lib/settings-ui.svelte';
  import { onMount } from 'svelte';
  import {
    commands,
    type ApplicationChoice,
    type FoundJob,
    type InboxLine,
    type MessageView
  } from '$lib/bindings';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import GoogleSignIn from '$lib/components/GoogleSignIn.svelte';
  import {
    dateWord,
    filterInbox,
    foundStatusWord,
    foundTitle,
    labelWord,
    syncSummary,
    type InboxFilter, needsSignIn } from '$lib/inbox';

  let lines = $state<InboxLine[]>([]);
  let found = $state<FoundJob[]>([]);
  let accounts = $state(0);
  let choices = $state<ApplicationChoice[]>([]);
  let filter = $state<InboxFilter>('all');
  let open = $state<MessageView | null>(null);
  let draft = $state('');
  let change = $state('');
  let loaded = $state(false);
  let busy = $state(false);
  let message = $state('');
  let error = $state('');

  const shown = $derived(filterInbox(lines, filter));
  const signInNeeded = $derived(needsSignIn(error));

  async function load() {
    try {
      const [l, a, c, f] = await Promise.all([
        commands.listInbox(),
        commands.listMailAccounts(),
        commands.listApplicationChoices(),
        commands.listFoundJobs()
      ]);
      if (f.status === 'ok') found = f.data;
      if (a.status === 'ok') accounts = a.data.length;
      if (c.status === 'ok') choices = c.data;
      if (l.status === 'ok') lines = l.data;
      else error = l.error;
    } catch (e) {
      error = `Running outside the desktop app: ${String(e)}`;
    }
    loaded = true;
  }
  onMount(load);

  async function show(id: number) {
    error = message = '';
    const r = await commands.openMessage(id);
    if (r.status !== 'ok') {
      error = r.error;
      return;
    }
    open = r.data;
    draft = r.data.draft ?? '';
    change = '';
    lines = lines.map((l) => (l.id === id ? { ...l, unread: false } : l));
  }

  async function decide(j: FoundJob, keep: boolean) {
    error = message = '';
    const r = keep
      ? await commands.confirmFoundJob(j.application_id)
      : await commands.dismissFoundJob(j.application_id);
    if (r.status !== 'ok') {
      error = r.error;
      return;
    }
    message = keep
      ? `Kept ${foundTitle(j)}. It is on your Applications board.`
      : `Dismissed ${foundTitle(j)}. CeeVee will not add it again.`;
    await load();
  }

  async function check() {
    busy = true;
    error = message = '';
    const r = await commands.syncMail();
    busy = false;
    if (r.status === 'ok') message = syncSummary(r.data);
    else error = r.error;
    await load();
  }

  async function link(e: { currentTarget: { value: string } }) {
    if (!open) return;
    const v = e.currentTarget.value;
    const r = await commands.linkMessage(open.line.id, v === '' ? null : Number(v));
    if (r.status === 'ok') await load();
    else error = r.error;
  }

  async function write(again: boolean) {
    if (!open) return;
    busy = true;
    error = message = '';
    const r = await commands.draftMailReply(open.line.id, '', again);
    busy = false;
    if (r.status !== 'ok') {
      error = r.error;
      return;
    }
    if (r.data.unverified.length > 0) {
      message = `Check these in the draft, they are not in your profile, the posting or the email: ${r.data.unverified.join(', ')}.`;
    }
    await show(open.line.id);
    await load();
  }

  async function saveDraft() {
    if (!open?.draft_id) return;
    const r = await commands.saveDocument(open.draft_id, draft);
    if (r.status === 'ok') message = 'Saved.';
    else error = r.error;
  }

  async function chatChange() {
    if (!open?.draft_id || change.trim() === '') return;
    busy = true;
    error = message = '';
    const r = await commands.chatEditDocument(open.draft_id, change, false);
    busy = false;
    if (r.status !== 'ok') {
      error = r.error;
      return;
    }
    if (r.data.applied) {
      draft = r.data.doc.content;
      change = '';
    } else {
      message = `Not changed. The new text mentions things you have not said: ${r.data.flagged.join(', ')}.`;
    }
  }

  async function copy() {
    try {
      await window.navigator.clipboard.writeText(draft);
      message = 'Copied. Paste it into a reply in your mail app.';
    } catch {
      error = 'Could not copy. Select the text and copy it yourself.';
    }
  }
</script>

<span class="label">Inbox</span>
{#if signInNeeded}
  <section class="signin" aria-label="Sign in again">
    <p><b>CeeVee needs you to sign in again to check your mail.</b> Your saved sign-in has expired or was removed.</p>
    <GoogleSignIn onchange={() => { error = ''; load().then(check); }} />
    <p class="note">Using another kind of account? <button class="link" onclick={() => openSettings('accounts')}>Reconnect it in Settings</button>.</p>
  </section>
{:else if error}<p role="alert">{error}</p>{/if}
{#if message}<p role="status">{message}</p>{/if}

{#if loaded && accounts === 0}
  <EmptyState title="Connect your email">
    <p>CeeVee reads replies from employers and keeps your applications up to date. Sign in with Google to start.</p>
    <GoogleSignIn onchange={() => load().then(check)} />
    <p class="note">Prefer another account? <button class="link" onclick={() => openSettings('accounts')}>Add it in Settings</button>.</p>
  </EmptyState>
{:else if loaded}
  {#if found.length > 0}
    <section class="found" aria-label="Jobs found in your email">
      <h2>{found.length === 1 ? '1 job found in your email' : `${found.length} jobs found in your email`}</h2>
      <p class="note">CeeVee added these from messages that looked like job updates or recruiter notes. Keep the ones that are real; dismissed ones are not added again.</p>
      <ul>
        {#each found as j (j.application_id)}
          <li>
            <div class="what">
              <b>{foundTitle(j)}</b>
              <span class="meta">
                {foundStatusWord(j.status)}{j.location ? ` · ${j.location}` : ''}{j.from ? ` · from ${j.from}` : ''}{j.received_at ? ` · ${dateWord(j.received_at)}` : ''}
              </span>
              {#if j.summary}<span class="meta">{j.summary}</span>{/if}
            </div>
            <div class="row2">
              {#if j.message_id != null}
                <button class="ghost" onclick={() => j.message_id != null && show(j.message_id)}>Read the email</button>
              {/if}
              <button onclick={() => decide(j, true)}>Keep</button>
              <button class="ghost" onclick={() => decide(j, false)}>Dismiss</button>
            </div>
          </li>
        {/each}
      </ul>
    </section>
  {/if}
  <div class="bar">
    <div class="tabs" role="group" aria-label="Show">
      <button class="ghost" aria-pressed={filter === 'all'} onclick={() => (filter = 'all')}>All</button>
      <button class="ghost" aria-pressed={filter === 'reply'} onclick={() => (filter = 'reply')}>Needs a reply</button>
      <button class="ghost" aria-pressed={filter === 'unread'} onclick={() => (filter = 'unread')}>Unread</button>
    </div>
    {#if !signInNeeded}<button class="ghost" onclick={check} disabled={busy}>{busy ? 'Checking' : 'Check mail now'}</button>{/if}
  </div>
  {#if lines.length === 0}
    <EmptyState title="No employer replies yet">
      <p>Replies from employers appear here. CeeVee checks your mail every 10 minutes.</p>
    </EmptyState>
  {:else}
    <div class="split">
      <ul class="list" aria-label="Messages">
        {#each shown as l (l.id)}
          <li>
            <button class="row" class:cur={open?.line.id === l.id} onclick={() => show(l.id)}>
              <span class="top">
                <span class="dot" class:on={l.unread}></span>
                <b class:unread={l.unread}>{l.from}</b>
                <span class="when">{dateWord(l.received_at)}</span>
              </span>
              <span class="subject">{l.subject}</span>
              <span class="meta">
                {labelWord(l.label)}{l.needs_reply ? ', needs a reply' : ''}{l.application ? ` · ${l.application}` : ''}
              </span>
            </button>
          </li>
        {/each}
        {#if shown.length === 0}<li class="empty">Nothing here.</li>{/if}
      </ul>
      <article class="detail" aria-live="polite">
        {#if open}
          <h2>{open.line.subject}</h2>
          <p class="note">From {open.line.from} ({open.from_address}) · {labelWord(open.line.label)}</p>
          {#if open.line.summary}<p>{open.line.summary}</p>{/if}
          {#if open.interview_at}<p><b>Interview time:</b> {open.interview_at}</p>{/if}
          <label class="link">
            <span class="label">Application</span>
            <select value={open.line.application_id ?? ''} onchange={link}>
              <option value="">Not linked</option>
              {#each choices as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
            </select>
          </label>
          <pre class="body">{open.body}</pre>
          <h3>Reply</h3>
          {#if open.draft_id}
            <textarea aria-label="Reply draft" rows="8" bind:value={draft}></textarea>
            <div class="row2">
              <button onclick={saveDraft}>Save changes</button>
              <button class="ghost" onclick={copy}>Copy reply</button>
              <button class="ghost" onclick={() => write(true)} disabled={busy}>Write a new draft</button>
            </div>
            <div class="row2">
              <input aria-label="Change the draft" placeholder="Ask for a change, for example make it shorter" bind:value={change} />
              <button class="ghost" onclick={chatChange} disabled={busy || change.trim() === ''}>Change</button>
            </div>
            <p class="note">CeeVee never sends mail for you. Copy the reply into your mail app when it is right.</p>
          {:else}
            <div class="row2"><button onclick={() => write(false)} disabled={busy}>{busy ? 'Writing' : 'Write a reply'}</button></div>
          {/if}
        {:else}
          <p class="note">Choose a message to read it.</p>
        {/if}
      </article>
    </div>
  {/if}
{/if}

<style>
  .signin { display: flex; flex-direction: column; gap: 8px; border: 1px solid var(--line); border-radius: var(--r-md); background: var(--surface); padding: 12px 16px; margin-bottom: 12px; }
  .signin p { margin: 0; }
  .found { border: 1px solid var(--line); background: var(--surface); padding: 24px; margin-bottom: 24px; display: flex; flex-direction: column; gap: 16px; }
  .found h2 { margin: 0; }
  .found ul { list-style: none; padding: 0; margin: 0; }
  .found li { display: flex; justify-content: space-between; align-items: center; gap: 16px; flex-wrap: wrap; padding: 16px 0; }
  .found li + li { border-top: 1px solid var(--line); }
  .what { display: flex; flex-direction: column; gap: 4px; min-width: 0; }
  .bar { display: flex; justify-content: space-between; gap: 16px; flex-wrap: wrap; }
  .tabs { display: flex; gap: 8px; }
  .tabs button[aria-pressed='true'] { color: var(--ink); background-size: 100% 1px; }
  .split { display: grid; grid-template-columns: minmax(260px, 340px) 1fr; gap: 24px; align-items: start; }
  .list { list-style: none; padding: 0; margin: 0; border: 1px solid var(--line); background: var(--surface); }
  .list li + li { border-top: 1px solid var(--line); }
  .row { all: unset; box-sizing: border-box; display: flex; flex-direction: column; gap: 4px; padding: 16px; width: 100%; cursor: pointer; }
  .row:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 4px; }
  .row.cur { background: var(--paper); }
  .top { display: flex; gap: 8px; align-items: center; }
  .when { margin-left: auto; font-size: 12px; }
  .unread { color: var(--ink); }
  .subject { color: var(--ink); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .meta { font-size: 12px; }
  .empty { padding: 16px; }
  .detail { border: 1px solid var(--line); background: var(--surface); padding: 24px; display: flex; flex-direction: column; gap: 16px; min-width: 0; }
  .body { white-space: pre-wrap; font: inherit; max-width: 34em; margin: 0; border-top: 1px solid var(--line); padding-top: 16px; }
  .link { display: flex; flex-direction: column; gap: 8px; align-items: flex-start; }
  select, textarea { font: inherit; background: var(--paper); color: var(--ink); border: 1px solid var(--border-control); padding: 12px 16px; max-width: 100%; }
  textarea { width: 100%; box-sizing: border-box; }
  .row2 { display: flex; gap: 8px; flex-wrap: wrap; }
  .row2 input { flex: 1; min-width: 160px; }
  h3 { margin: 0; }
  b { color: var(--ink); font-weight: 500; }
  @media (max-width: 900px) { .split { grid-template-columns: 1fr; } }
</style>
