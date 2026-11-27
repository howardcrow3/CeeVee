<script lang="ts">
  import { onMount } from 'svelte';
  import { commands, type MeetingLine, type PersonLine } from '$lib/bindings';
  import { localToUtc, whenText } from '$lib/people';

  let { people }: { people: PersonLine[] } = $props();

  let meetings = $state<MeetingLine[]>([]);
  let error = $state('');
  let adding = $state(false);
  let person = $state(0);
  let title = $state('Coffee');
  let when = $state('');
  let minutes = $state(30);
  let prep = $state<{ id: number; text: string } | null>(null);
  let notesFor = $state<number | null>(null);
  let notes = $state('');

  async function load() {
    try {
      const r = await commands.listMeetings();
      if (r.status === 'ok') meetings = r.data;
      else error = r.error;
    } catch (e) {
      error = String(e);
    }
  }
  onMount(load);

  async function plan() {
    error = '';
    const at = localToUtc(when);
    if (!at) {
      error = 'Pick a date and time.';
      return;
    }
    const r = await commands.planMeeting(person || people[0]?.id, title, at, minutes);
    if (r.status === 'ok') {
      adding = false;
      when = '';
      await load();
    } else error = r.error;
  }

  async function showPrep(id: number) {
    if (prep?.id === id) {
      prep = null;
      return;
    }
    const r = await commands.meetingPrep(id);
    if (r.status === 'ok') prep = { id, text: r.data };
    else error = r.error;
  }

  async function calendar(id: number) {
    const r = await commands.meetingCalendarFile(id);
    if (r.status !== 'ok') {
      error = r.error;
      return;
    }
    // eslint-disable-next-line no-undef
    const url = URL.createObjectURL(new Blob([r.data], { type: 'text/calendar' }));
    const a = document.createElement('a');
    a.href = url;
    a.download = 'meeting.ics';
    a.click();
    // eslint-disable-next-line no-undef
    URL.revokeObjectURL(url);
  }

  async function finish(id: number) {
    const r = await commands.finishMeeting(id, notes);
    notesFor = null;
    notes = '';
    if (r.status !== 'ok') error = r.error;
    await load();
  }

  async function cancel(id: number) {
    const r = await commands.cancelMeeting(id);
    if (r.status !== 'ok') error = r.error;
    await load();
  }
</script>

<section>
  <h2>Meetings</h2>
  {#if error}<p role="alert">{error}</p>{/if}
  {#if people.length > 0}
    {#if adding}
      <form onsubmit={(e) => { e.preventDefault(); plan(); }}>
        <label>With
          <select bind:value={person}>
            {#each people as p (p.id)}<option value={p.id}>{p.name}</option>{/each}
          </select>
        </label>
        <label>What <input bind:value={title} required /></label>
        <label>When <input type="datetime-local" bind:value={when} required /></label>
        <label>Minutes <input type="number" min="5" max="480" step="5" bind:value={minutes} /></label>
        <div class="row">
          <button type="submit">Plan meeting</button>
          <button type="button" class="ghost" onclick={() => (adding = false)}>Cancel</button>
        </div>
      </form>
    {:else}
      <button onclick={() => { person = people[0].id; adding = true; }}>Plan a meeting</button>
    {/if}
  {/if}
  <ul>
    {#each meetings as m (m.id)}
      <li>
        <div class="who">
          <b>{m.title} with {m.person}</b>
          <span class="muted">{whenText(m.starts_at)} · {m.minutes} min</span>
        </div>
        <div class="row">
          <button class="ghost" onclick={() => showPrep(m.id)}>Prep</button>
          <button class="ghost" onclick={() => calendar(m.id)}>Add to calendar</button>
          <button onclick={() => (notesFor = notesFor === m.id ? null : m.id)}>Done</button>
          <button class="ghost" onclick={() => cancel(m.id)}>Cancel</button>
        </div>
        {#if prep?.id === m.id}<pre>{prep.text}</pre>{/if}
        {#if notesFor === m.id}
          <form onsubmit={(e) => { e.preventDefault(); finish(m.id); }}>
            <label>What came of it? <input bind:value={notes} placeholder="Optional" /></label>
            <button type="submit">Save</button>
          </form>
        {/if}
      </li>
    {/each}
  </ul>
</section>

<style>
  section { margin-top: 24px; max-width: 760px; }
  h2 { margin: 0 0 8px; }
  ul { list-style: none; padding: 0; margin: 12px 0 0; border: 1px solid var(--line); background: var(--surface); }
  ul:empty { display: none; }
  li { padding: 8px 12px; border-bottom: 1px solid var(--line); display: flex; flex-direction: column; gap: 8px; }
  li:last-child { border-bottom: none; }
  .who { display: flex; flex-direction: column; }
  .row { display: flex; gap: 8px; flex-wrap: wrap; }
  form { display: flex; flex-direction: column; gap: 8px; max-width: 420px; margin-top: 8px; }
  label { display: flex; flex-direction: column; gap: 4px; }
  pre { margin: 0; white-space: pre-wrap; font: inherit; }
  b { color: var(--ink); font-weight: 500; }
  .muted { opacity: 0.8; }
  input, select { font: inherit; background: var(--paper); color: var(--ink); border: 1px solid var(--border-control); padding: 6px 10px; }
</style>
