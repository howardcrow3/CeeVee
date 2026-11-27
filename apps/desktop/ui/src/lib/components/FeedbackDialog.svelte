<script lang="ts">
  /* global HTMLDialogElement */
  // "Send feedback": stars, a category, the user's words, an optional reply address (prefilled from the
  // profile or a connected mail account) and "Send anonymously", which keeps the address from being sent.
  // The Rust side signs and sends it to the High Turtle feedback service.
  import { commands } from '$lib/bindings';
  import { MAX_FEEDBACK, charsLeftText, feedbackCategories, feedbackEmailOk, feedbackInput, feedbackReady, type FeedbackCategory } from '$lib/feedback';

  let { open = $bindable(false) }: { open?: boolean } = $props();

  let dialog: HTMLDialogElement | undefined = $state();
  let rating = $state(0);
  let category = $state<FeedbackCategory>('feature');
  let message = $state('');
  let email = $state('');
  let anonymous = $state(false);
  let busy = $state(false);
  let error = $state('');
  let sent = $state(false);

  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) {
      sent = false;
      error = '';
      dialog.showModal();
      if (!email) prefillEmail();
    } else if (!open && dialog.open) dialog.close();
  });

  async function prefillEmail() {
    try {
      const r = await commands.feedbackEmail();
      if (r.status === 'ok' && r.data && !email) email = r.data;
    } catch {
      // Older builds or no profile: leave it blank.
    }
  }

  async function send() {
    error = '';
    if (!anonymous && !feedbackEmailOk(email)) {
      error = 'That email address doesn’t look right. Fix it, clear it, or tick Send anonymously.';
      return;
    }
    busy = true;
    try {
      const r = await commands.sendFeedback(feedbackInput(rating, category, message, email, anonymous));
      if (r.status === 'ok') {
        sent = true;
        rating = 0;
        message = '';
        anonymous = false;
      } else error = r.error;
    } catch (e) {
      error = String(e);
    }
    busy = false;
  }
</script>

<dialog bind:this={dialog} onclose={() => (open = false)} aria-labelledby="fb-title">
  {#if sent}
    <h2 id="fb-title">Thank you</h2>
    <p>We read every message. It helps decide what CeeVee does next.</p>
    <div class="actions"><button onclick={() => (open = false)}>Close</button></div>
  {:else}
    <h2 id="fb-title">Send feedback</h2>
    <p class="note">Tell us what to add, what went wrong or what you like. It goes straight to the people who make CeeVee.</p>

    <span class="label" id="fb-rate">How do you like CeeVee?</span>
    <div class="stars" role="radiogroup" aria-labelledby="fb-rate">
      {#each [1, 2, 3, 4, 5] as n (n)}
        <button type="button" role="radio" aria-checked={rating === n} aria-label={`${n} star${n > 1 ? 's' : ''}`}
          class:on={n <= rating} onclick={() => (rating = rating === n ? 0 : n)}>★</button>
      {/each}
    </div>

    <span class="label">What's it about?</span>
    <div class="cats">
      {#each feedbackCategories as c (c.value)}
        <label><input type="radio" name="fb-cat" value={c.value} bind:group={category} /> {c.label}</label>
      {/each}
    </div>

    <label class="label" for="fb-msg">Your feedback</label>
    <textarea id="fb-msg" rows="6" maxlength={MAX_FEEDBACK} bind:value={message}
      placeholder="A feature you'd like, a problem you hit, or anything else"></textarea>
    {#if charsLeftText(message)}<p class="note">{charsLeftText(message)}</p>{/if}

    <label class="label" for="fb-email">Email (optional, so we can reply)</label>
    <input id="fb-email" type="email" autocomplete="email" bind:value={email} disabled={anonymous} />
    <label class="anon"><input type="checkbox" bind:checked={anonymous} /> Send anonymously (your email isn't sent)</label>

    {#if error}<p role="alert" class="err">{error}</p>{/if}
    <div class="actions">
      <button class="ghost" onclick={() => (open = false)}>Cancel</button>
      <button onclick={send} disabled={busy || !feedbackReady(rating, message)}>{busy ? 'Sending' : 'Send'}</button>
    </div>
  {/if}
</dialog>

<style>
  dialog { width: min(560px, calc(100vw - 32px)); border: 1px solid var(--line); background: var(--paper); color: var(--body);
    padding: 24px; border-radius: var(--r-lg); box-shadow: var(--shadow-pop); }
  dialog[open] { display: flex; flex-direction: column; gap: 12px; }
  dialog::backdrop { background: rgb(28 28 28 / 0.45); }
  .note { font-size: 13px; }
  .stars { display: flex; gap: 4px; }
  .stars button { width: 40px; height: 40px; padding: 0; font-size: 20px; letter-spacing: 0; background: transparent;
    color: var(--line); border-color: var(--line); }
  .stars button.on { color: var(--accent); border-color: var(--accent); }
  .stars button:hover:not(:disabled) { background: transparent; color: var(--accent); border-color: var(--accent); }
  .cats { display: flex; gap: 16px; flex-wrap: wrap; }
  .cats label { display: flex; align-items: center; gap: 6px; cursor: pointer; }
  .cats input { accent-color: var(--ink); }
  textarea, input[type='email'] { width: 100%; font: 400 14px/1.5 var(--font-text); color: var(--ink); background: var(--paper);
    border: 1px solid var(--border-control); border-radius: var(--r-sm); padding: 6px 10px; }
  textarea { resize: vertical; }
  input[type='email']:disabled { opacity: 0.5; }
  .anon { display: flex; align-items: center; gap: 6px; cursor: pointer; font-size: 13px; }
  .anon input { accent-color: var(--ink); }
  .err { color: var(--danger); }
  .actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 8px; }
</style>
