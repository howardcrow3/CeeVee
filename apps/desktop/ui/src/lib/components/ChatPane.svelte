<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { resolve } from '$app/paths';
  import { commands } from '$lib/bindings';
  import { actionNotes, buildPrompt, wantsAction, focusLine, loadChatContext, contextChip, contextSummary, friendlyError, suggestions, type ChatContext, type Turn } from '$lib/chat';
  import { openSettings } from '$lib/settings-ui.svelte';

  let { focus = '', compact = false }: { focus?: string; compact?: boolean } = $props();
  let context = $state<ChatContext>({ jobs: [], applications: [], jobTypes: [], waiting: 0 });
  let turns = $state<(Turn & { needsModel?: boolean; notes?: string[] })[]>([]);
  let draft = $state('');
  let thinking = $state(false);
  let list: HTMLDivElement | undefined = $state();

  onMount(async () => {
    context = await loadChatContext();
  });

  async function send(text: string) {
    const message = text.trim();
    if (message === '' || thinking) return;
    const history = turns.map(({ role, text: t }) => ({ role, text: t }));
    turns = [...turns, { role: 'user', text: message }];
    draft = '';
    thinking = true;
    await tick();
    list?.scrollTo({ top: list.scrollHeight });
    try {
      if (wantsAction(message)) {
        const a = await commands.chatAct(message);
        if (a.status === 'ok') {
          turns = [...turns, { role: 'assistant', text: a.data.reply, notes: actionNotes(a.data.did, a.data.waiting) }];
          context = await loadChatContext();
          return;
        }
        const f = friendlyError(a.error);
        turns = [...turns, { role: 'assistant', text: f.text, needsModel: f.needsModel }];
        return;
      }
      const r = await commands.askModel(buildPrompt(contextSummary(context) + focusLine(focus), history, message));
      if (r.status === 'ok') turns = [...turns, { role: 'assistant', text: r.data.trim() || 'I have nothing to add.' }];
      else {
        const f = friendlyError(r.error);
        turns = [...turns, { role: 'assistant', text: f.text, needsModel: f.needsModel }];
      }
    } catch (e) {
      turns = [...turns, { role: 'assistant', text: `Running outside the desktop app: ${String(e)}` }];
    } finally {
      thinking = false;
      await tick();
      list?.scrollTo({ top: list.scrollHeight });
    }
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      send(draft);
    }
  }
</script>

<div class="chat" class:compact>
  {#if !compact}<span class="label">Chat</span>{/if}
  <div class="scroll" bind:this={list}>
    {#if turns.length === 0}
      <div class="hello">
        <h1>What would you like to do?</h1>
        {#if context.waiting > 0}
          <p><span class="dot on"></span>{context.waiting} {context.waiting === 1 ? 'item is' : 'items are'} waiting for your approval. <a href={resolve('/applications')}>See them</a></p>
        {:else if context.jobTypes.length === 0}
          <p class="note">Add your resume and a <a href={resolve('/job-types')}>job type</a>, and CeeVee starts searching. <a href={resolve('/welcome')}>Walk me through it</a>.</p>
        {/if}
        <div class="chips">
          {#each suggestions(context) as s (s)}
            <button class="chip" onclick={() => send(s)}>{s}</button>
          {/each}
        </div>
      </div>
    {:else}
      <ol class="turns" aria-live="polite">
        {#each turns as t, i (i)}
          <li class={t.role}>
            <span class="who">{t.role === 'user' ? 'You' : 'CeeVee'}</span>
            <p>{t.text}</p>
            {#if t.notes}{#each t.notes as n (n)}<span class="note">{n}</span>{/each}{/if}
            {#if t.needsModel}<button class="link" onclick={() => openSettings('general')}>Download it in Settings</button>{/if}
          </li>
        {/each}
        {#if thinking}<li class="assistant"><span class="who">CeeVee</span><p class="note">Thinking</p></li>{/if}
      </ol>
    {/if}
  </div>
  <div class="composer">
    <textarea aria-label="Message CeeVee" rows="2" placeholder={focus ? 'Ask about this' : 'Ask about your jobs and applications'} bind:value={draft} onkeydown={onKey}></textarea>
    <div class="bar">
      <span class="note">{contextChip(context)}. Conversations are not saved yet.</span>
      <button onclick={() => send(draft)} disabled={thinking || draft.trim() === ''}>Send</button>
    </div>
  </div>
</div>

<style>
  .chat { display: flex; flex-direction: column; height: calc(100vh - 48px); min-height: 360px; gap: 12px; }
  .chat.compact { height: calc(100vh - 120px); gap: 8px; }
  .compact .hello { margin-top: 24px; }
  .compact .turns p { max-width: 100%; }
  .scroll { flex: 1; overflow-y: auto; min-height: 0; }
  .hello { max-width: 560px; margin: 12vh auto 0; display: flex; flex-direction: column; gap: 12px; }
  .hello h1 { margin: 0; }
  .hello p { margin: 0; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; }
  .chip { height: 22px; padding: 0 10px; border-radius: var(--r-pill); background: var(--surface); color: var(--body); border: 1px solid var(--line); font-size: 12px; }
  .chip:hover:not(:disabled) { background: var(--hover); color: var(--ink); }
  .turns { list-style: none; margin: 0 auto; padding: 0; max-width: 720px; display: flex; flex-direction: column; gap: 12px; }
  .turns li { display: flex; flex-direction: column; gap: 2px; align-items: flex-start; }
  .turns li.user { align-items: flex-end; }
  .turns p { margin: 0; white-space: pre-wrap; padding: 8px 12px; border-radius: var(--r-md); border: 1px solid var(--line); background: var(--surface); max-width: 90%; }
  .turns li.user p { background: var(--selected); }
  .who { font-size: 11px; color: var(--muted); padding: 0 4px; }
  .composer { max-width: 720px; width: 100%; margin: 0 auto; display: flex; flex-direction: column; gap: 6px; border: 1px solid var(--border-control); border-radius: var(--r-md); background: var(--surface); padding: 8px; }
  .composer textarea { border: 0; background: transparent; resize: none; padding: 4px; font: inherit; color: var(--ink); width: 100%; box-sizing: border-box; }
  .composer textarea:focus { outline: none; }
  .composer:focus-within { outline: 2px solid var(--focus-ring); outline-offset: 2px; }
  .bar { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
</style>
