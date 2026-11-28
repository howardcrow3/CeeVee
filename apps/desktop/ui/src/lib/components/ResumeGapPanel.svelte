<script lang="ts">
  import { commands, type ResumeGap } from '$lib/bindings';
  import { gapGroups } from '$lib/jobs';

  let { job }: { job: number } = $props();

  let gap = $state<ResumeGap | null>(null);
  let busy = $state(false);
  let error = $state('');
  let forJob = $state<number | null>(null);

  // A different job starts with no result.
  $effect(() => {
    if (forJob !== job) {
      gap = null;
      error = '';
      forJob = job;
    }
  });

  async function check() {
    busy = true;
    error = '';
    const r = await commands.resumeGap(job);
    busy = false;
    if (r.status === 'ok') gap = r.data;
    else error = r.error;
  }
  const groups = $derived(gap ? gapGroups(gap.terms) : null);
</script>

<section aria-label="Resume check">
  <div class="head">
    <b>Resume check</b>
    <button class="ghost" disabled={busy} onclick={check}>{busy ? 'Checking' : gap ? 'Check again' : 'Check my resume'}</button>
  </div>
  {#if error}<p role="alert">{error}</p>{/if}
  {#if gap && groups}
    {#if gap.resume === ''}
      <p class="note">Add a resume in Documents first, then check it against this job.</p>
    {:else if gap.terms.length === 0}
      <p class="note">No specific skills or tools were found in this posting to check against {gap.resume}.</p>
    {:else}
      <p class="note">Checked against {gap.resume}.</p>
      {#if groups.add.length}<p><b>In your profile but not on the resume:</b> {groups.add.join(', ')}. Ask CeeVee to tailor the resume to add them.</p>{/if}
      {#if groups.ask.length}<p><b>Asked for, not in your profile:</b> {groups.ask.join(', ')}. Only add these if you really have them.</p>{/if}
      {#if groups.covered.length}<p><b>Already on the resume:</b> {groups.covered.join(', ')}.</p>{/if}
    {/if}
  {/if}
</section>

<style>
  section { border-top: 1px solid var(--line); padding: 8px 0; }
  .head { display: flex; justify-content: space-between; align-items: baseline; gap: 8px; }
  p { margin: 6px 0; }
</style>
