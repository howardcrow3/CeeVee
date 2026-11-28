<script lang="ts">
  import { commands, type PracticeNote } from '$lib/bindings';

  let { job }: { job: number } = $props();

  let questions = $state<string[]>([]);
  let picked = $state('');
  let answer = $state('');
  let feedback = $state('');
  let notes = $state<PracticeNote[]>([]);
  let busy = $state<'' | 'questions' | 'feedback'>('');
  let error = $state('');
  let forJob = $state<number | null>(null);

  // A different job starts clean, with its own saved notes.
  $effect(() => {
    if (forJob !== job) {
      forJob = job;
      questions = [];
      picked = '';
      answer = '';
      feedback = '';
      error = '';
      loadNotes(job);
    }
  });

  async function loadNotes(id: number) {
    const r = await commands.practiceNotes(id);
    if (r.status === 'ok' && id === forJob) notes = r.data;
  }

  async function getQuestions() {
    busy = 'questions';
    error = '';
    const r = await commands.practiceQuestions(job);
    busy = '';
    if (r.status === 'ok') {
      questions = r.data;
      picked = r.data[0] ?? '';
      answer = '';
      feedback = '';
    } else error = r.error;
  }

  async function getFeedback() {
    busy = 'feedback';
    error = '';
    const r = await commands.practiceFeedback(job, picked, answer);
    busy = '';
    if (r.status === 'ok') feedback = r.data;
    else error = r.error;
  }

  async function keep() {
    const r = await commands.practiceSave(job, picked, answer, feedback);
    if (r.status === 'ok') {
      answer = '';
      feedback = '';
      await loadNotes(job);
    } else error = r.error;
  }

  async function remove(id: number) {
    const r = await commands.practiceDelete(id);
    if (r.status === 'ok') await loadNotes(job);
    else error = r.error;
  }
</script>

<section aria-label="Interview practice">
  <div class="head">
    <b>Interview practice</b>
    <button class="ghost" disabled={busy !== ''} onclick={getQuestions}>{busy === 'questions' ? 'Thinking' : questions.length ? 'New questions' : 'Practice for this job'}</button>
  </div>
  {#if error}<p role="alert">{error}</p>{/if}
  {#if questions.length}
    <label>Question
      <select bind:value={picked}>
        {#each questions as q (q)}<option value={q}>{q}</option>{/each}
      </select>
    </label>
    <label>Your answer
      <textarea rows="4" bind:value={answer} placeholder="Write it the way you would say it"></textarea>
    </label>
    <div class="acts">
      <button class="ghost" disabled={busy !== '' || answer.trim() === ''} onclick={getFeedback}>{busy === 'feedback' ? 'Reading' : 'Get feedback'}</button>
      {#if feedback}<button class="ghost" onclick={keep}>Keep this note</button>{/if}
    </div>
    {#if feedback}<p class="note">{feedback}</p>{/if}
  {/if}
  {#if notes.length}
    <details>
      <summary>Kept notes ({notes.length})</summary>
      {#each notes as n (n.id)}
        <div class="n">
          <b>{n.question}</b>
          <p>{n.answer}</p>
          {#if n.feedback}<p class="note">{n.feedback}</p>{/if}
          <button class="ghost" onclick={() => remove(n.id)}>Remove</button>
        </div>
      {/each}
    </details>
  {/if}
</section>

<style>
  section { border-top: 1px solid var(--line); padding: 8px 0; }
  .head { display: flex; justify-content: space-between; align-items: baseline; gap: 8px; }
  label { display: block; margin: 6px 0; }
  select, textarea { width: 100%; margin-top: 2px; }
  .acts { display: flex; gap: 8px; flex-wrap: wrap; }
  p { margin: 6px 0; }
  .n { border-top: 1px solid var(--line); padding: 6px 0; }
</style>
