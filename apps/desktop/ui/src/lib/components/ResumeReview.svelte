<script lang="ts">
  import { diffLines, diffSummary, isFlagged } from '$lib/diff';

  let {
    before,
    after,
    flagged,
    content = $bindable(),
    onaccept,
    onthrow
  }: {
    before: string;
    after: string;
    flagged: string[];
    content: string;
    onaccept: () => void;
    onthrow: () => void;
  } = $props();

  let editing = $state(false);
  const lines = $derived(diffLines(before, after));
  const changed = $derived(lines.filter((l) => l.kind !== 'same'));
</script>

<section class="review" aria-label="Review the tailored resume">
  <h2>Your tailored resume</h2>
  <p class="note">{diffSummary(lines)}</p>
  {#if flagged.length}
    <p class="note warn">Check the amber lines. They say something your profile and the posting do not: {flagged.join(', ')}.</p>
  {:else}
    <p class="note">Everything in this draft matches your profile and the posting.</p>
  {/if}
  {#if changed.length}
    <ul class="changes" aria-label="Changes">
      {#each changed as l, i (`${i}:${l.kind}:${l.text}`)}
        <li class:add={l.kind === 'add'} class:del={l.kind === 'del'} class:flag={l.kind === 'add' && isFlagged(l.text, flagged)}>
          <span class="mark" aria-hidden="true">{l.kind === 'add' ? '+' : '−'}</span>{l.text}
        </li>
      {/each}
    </ul>
  {/if}
  <div class="row">
    <button onclick={onaccept}>Accept all and save</button>
    <button class="ghost" onclick={() => (editing = !editing)}>{editing ? 'Hide text' : 'Edit text'}</button>
    <button class="ghost" onclick={onthrow}>Throw away</button>
  </div>
  {#if editing}
    <textarea aria-label="Draft" rows="14" bind:value={content}></textarea>
  {/if}
</section>

<style>
  .review { display: flex; flex-direction: column; gap: 10px; max-width: 760px; margin: 16px 0; }
  .changes { list-style: none; margin: 0; padding: 0; border: 1px solid var(--line); }
  .changes li { padding: 4px 10px; display: flex; gap: 8px; }
  .changes li.add { background: color-mix(in srgb, var(--accent, #2a7) 14%, transparent); }
  .changes li.del { color: var(--muted, #777); text-decoration: line-through; }
  .changes li.flag { background: color-mix(in srgb, #d90 25%, transparent); }
  .mark { width: 1em; flex-shrink: 0; }
  .warn { color: var(--ink); }
  .row { display: flex; gap: 8px; }
  textarea { font: inherit; background: var(--paper); color: var(--ink); border: 1px solid var(--border-control); border-radius: var(--r-sm); padding: 8px 12px; }
</style>
