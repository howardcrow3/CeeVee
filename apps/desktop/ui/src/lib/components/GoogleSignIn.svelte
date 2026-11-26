<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { commands, type MailAccountLine } from '$lib/bindings';
  import { googleAccount } from '$lib/inbox';

  let { onchange }: { onchange?: () => void } = $props();

  let available = $state(false);
  let accounts = $state<MailAccountLine[]>([]);
  let busy = $state(false);
  let error = $state('');
  let timer: number | undefined;

  const signedIn = $derived(googleAccount(accounts));

  async function load() {
    const r = await commands.listMailAccounts();
    if (r.status === 'ok') accounts = r.data;
  }
  // Read the accounts again while waiting and whenever the window returns to the front, so the
  // button never shows a stale state after the browser step finished.
  const refresh = () => load().catch(() => {});

  onMount(() => {
    commands.googleAvailable().then((g) => (available = g)).catch(() => {});
    refresh();
    window.addEventListener('focus', refresh);
    timer = window.setInterval(() => {
      if (busy) refresh();
    }, 2000);
  });
  onDestroy(() => {
    if (typeof window === 'undefined') return;
    window.removeEventListener('focus', refresh);
    window.clearInterval(timer);
  });

  async function signIn() {
    busy = true;
    error = '';
    try {
      const r = await commands.connectGoogle();
      if (r.status !== 'ok') error = r.error;
    } catch (e) {
      error = `Google sign-in stopped unexpectedly: ${String(e)}`;
    } finally {
      busy = false;
      await refresh();
      onchange?.();
    }
  }
  async function cancel() {
    await commands.cancelGoogleSignIn();
  }
</script>

{#if available}
  <div class="box">
    <p class="note">
      Optional. Sign in with Google and CeeVee can read replies from recruiters in your Gmail and keep your applications up to
      date. It asks for read-only access: it cannot send, change or delete anything, and you can remove access at any time in
      your Google account.
    </p>
    {#if signedIn}<p><b>Signed in with Google</b> as {signedIn.address}.</p>{/if}
    <div class="row">
      {#if busy}
        <span class="note">Waiting for Google. Finish in your browser, or cancel to start over.</span>
        <button class="ghost" onclick={cancel}>Cancel</button>
      {:else}
        <button class="gbtn" onclick={signIn}>
          <svg viewBox="0 0 48 48" width="16" height="16" aria-hidden="true">
            <path fill="#EA4335" d="M24 9.5c3.5 0 6.6 1.2 9.1 3.6l6.8-6.8C35.8 2.4 30.3 0 24 0 14.6 0 6.5 5.4 2.6 13.2l7.9 6.1C12.4 13.6 17.7 9.5 24 9.5z" />
            <path fill="#4285F4" d="M46.5 24.5c0-1.6-.1-3.1-.4-4.5H24v9h12.7c-.6 3-2.3 5.5-4.8 7.2l7.5 5.8c4.4-4.1 7.1-10.1 7.1-17.5z" />
            <path fill="#FBBC05" d="M10.5 28.7c-.5-1.4-.8-3-.8-4.7s.3-3.2.8-4.7l-7.9-6.1C.9 16.4 0 20.1 0 24s.9 7.6 2.6 10.8l7.9-6.1z" />
            <path fill="#34A853" d="M24 48c6.5 0 11.900-2.1 15.900-5.800l-7.500-5.800c-2.100 1.400-4.800 2.300-8.400 2.300-6.300 0-11.600-4.100-13.500-9.800l-7.900 6.100C6.500 42.600 14.600 48 24 48z" />
          </svg>
          {signedIn ? 'Sign in with a different Google account' : 'Sign in with Google'}
        </button>
      {/if}
    </div>
    {#if error}<p role="alert">{error}</p>{/if}
  </div>
{/if}

<style>
  .box { display: flex; flex-direction: column; gap: 8px; }
  .row { display: flex; gap: 8px; flex-wrap: wrap; align-items: center; }
  b { color: var(--ink); font-weight: 500; }
  /* Google's sign-in button: white with a grey outline in light, near black in dark. */
  .gbtn { --g-bg: #fff; --g-line: #747775; --g-text: #1f1f1f; display: inline-flex; align-items: center; gap: 8px; height: 32px;
    padding: 0 14px; background: var(--g-bg); color: var(--g-text); border: 1px solid var(--g-line); border-radius: var(--r-pill); font-weight: 500; }
  .gbtn:hover:not(:disabled) { background: var(--g-bg); color: var(--g-text); box-shadow: 0 1px 2px rgba(60, 64, 67, 0.3); }
  :global(:root[data-theme='dark']) .gbtn { --g-bg: #131314; --g-line: #8e918f; --g-text: #e3e3e3; }
  @media (prefers-color-scheme: dark) {
    :global(:root:not([data-theme='light'])) .gbtn { --g-bg: #131314; --g-line: #8e918f; --g-text: #e3e3e3; }
  }
</style>
