<script lang="ts">
  import { commands, type IntroLine } from '$lib/bindings';
  import { tierLabel } from '$lib/people';

  let { job }: { job: number } = $props();

  let paths = $state<IntroLine[] | null>(null);
  let error = $state('');
  let message = $state<{ person: number; name: string; text: string } | null>(null);
  let sent = $state('');

  // A different job means a fresh panel.
  $effect(() => {
    void job;
    paths = null;
    message = null;
    sent = '';
    error = '';
  });

  async function find() {
    const r = await commands.introPaths(job);
    if (r.status === 'ok') paths = r.data;
    else error = r.error;
  }

  async function write(p: IntroLine, ask: string) {
    sent = '';
    const r = await commands.introDraft(job, p.id, ask);
    if (r.status === 'ok') message = { person: p.id, name: p.name, text: r.data };
    else error = r.error;
  }

  async function copyAndRecord() {
    if (!message) return;
    try {
      // eslint-disable-next-line no-undef
      await navigator.clipboard.writeText(message.text);
    } catch {
      /* the person can still select the text */
    }
    const r = await commands.recordIntroAsk(job, message.person, message.text);
    if (r.status === 'ok') {
      sent = `Copied. Marked as asked, so ${message.name} moves down the list.`;
      message = null;
      await find();
    } else error = r.error;
  }

  function note(p: IntroLine): string {
    const bits = [tierLabel(p.tier)];
    if (p.former) bits.push('used to work there');
    else if (p.title) bits.push(p.title);
    if (p.asked) bits.push(p.asked);
    return bits.join(' · ');
  }
</script>

{#if paths === null}
  <button class="link" onclick={find}>Find an introduction</button>
{:else if paths.length === 0}
  <div class="note">You do not know anyone here yet. Import contacts or your LinkedIn connections on the Network page.</div>
{:else}
  <ul class="people">
    {#each paths as p (p.id)}
      <li>
        <b>{p.name}</b> <span class="note">{note(p)}</span>
        <div class="acts">
          <button class="ghost" onclick={() => write(p, 'referral')}>Ask for a referral</button>
          <button class="ghost" onclick={() => write(p, 'chat')}>Ask for a chat</button>
        </div>
      </li>
    {/each}
  </ul>
{/if}
{#if message}
  <label class="note">Message to {message.name}. Change it, then send it yourself.
    <textarea rows="9" bind:value={message.text}></textarea>
  </label>
  <div class="acts">
    <button onclick={copyAndRecord}>Copy and mark as asked</button>
    <button class="ghost" onclick={() => (message = null)}>Cancel</button>
  </div>
{/if}
{#if sent}<div class="note" role="status">{sent}</div>{/if}
{#if error}<div class="note" role="alert">{error}</div>{/if}

<style>
  .people { margin: 4px 0 0; padding-left: 0; list-style: none; display: flex; flex-direction: column; gap: 8px; }
  .acts { display: flex; gap: 6px; flex-wrap: wrap; margin-top: 4px; }
  textarea { width: 100%; font: inherit; background: var(--paper); color: var(--ink); border: 1px solid var(--border-control); padding: 6px 8px; }
  b { color: var(--ink); font-weight: 500; }
  .note { font-size: 13px; color: var(--body); }
</style>
