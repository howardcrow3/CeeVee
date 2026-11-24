<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { commands, type MailAccountLine } from '$lib/bindings';
  import { googleAccount, looksLikeAddress, syncSummary } from '$lib/inbox';

  let accounts = $state<MailAccountLine[]>([]);
  let address = $state('');
  let host = $state('');
  let port = $state(993);
  let password = $state('');
  let busy = $state(false);
  let google = $state(false);
  let message = $state('');
  let error = $state('');

  async function load() {
    const r = await commands.listMailAccounts();
    if (r.status === 'ok') accounts = r.data;
    else error = r.error;
  }
  // While waiting for Google, and whenever the window comes back to the front, read the accounts
  // again so Settings never shows a stale "Sign in with Google" after the browser step finished.
  let timer: number | undefined;
  const refresh = () => load().catch(() => {});
  onMount(() => {
    load().catch((e) => (error = `Running outside the desktop app: ${String(e)}`));
    commands.googleAvailable().then((g) => (google = g)).catch(() => {});
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

  async function fillServer() {
    if (!looksLikeAddress(address) || host) return;
    const p = await commands.imapPreset(address);
    if (p) {
      host = p.host;
      port = p.port;
    }
  }

  async function connect() {
    busy = true;
    error = message = '';
    const r = await commands.connectImap(address, host, port, password);
    busy = false;
    if (r.status !== 'ok') {
      error = r.error;
      return;
    }
    password = '';
    address = host = '';
    message = `Connected ${r.data.address}. Choose Check mail now to read your inbox.`;
    await load();
  }

  async function signInWithGoogle() {
    busy = true;
    error = message = '';
    try {
      const r = await commands.connectGoogle();
      if (r.status !== 'ok') {
        error = r.error;
        return;
      }
      message = `Connected ${r.data.address}. Choose Check mail now to read your inbox.`;
    } catch (e) {
      error = `Google sign-in stopped unexpectedly: ${String(e)}`;
    } finally {
      busy = false;
      await refresh();
    }
  }

  async function cancelGoogle() {
    await commands.cancelGoogleSignIn();
  }

  let signedIn = $derived(googleAccount(accounts));

  async function disconnect(a: MailAccountLine) {
    error = message = '';
    const r = await commands.disconnectMailAccount(a.id);
    if (r.status === 'ok') message = `Disconnected ${a.address}.`;
    else error = r.error;
    // The account is gone locally even when Google could not be reached, so always refresh.
    await load();
  }

  async function syncNow() {
    busy = true;
    error = message = '';
    const r = await commands.syncMail();
    busy = false;
    if (r.status === 'ok') message = syncSummary(r.data);
    else error = r.error;
    await load();
  }
</script>

<section>
  <h2>Email</h2>
  <p class="note">
    Connect your email and CeeVee reads replies from employers, links them to your applications and
    keeps their status up to date. It reads only messages from companies you apply to, from job
    systems such as Greenhouse and Lever, and job alerts. It never changes or deletes mail, and
    your password stays in this computer's keychain.
  </p>
  {#each accounts as a (a.id)}
    <div class="item">
      <div>
        <b>{a.address}</b>
        <div class="note">{a.last_synced_at ? `Checked ${a.last_synced_at}` : 'Not checked yet'}</div>
      </div>
      <button class="ghost" onclick={() => disconnect(a)}>Disconnect</button>
    </div>
  {/each}
  {#if accounts.length > 0}
    <div class="row"><button onclick={syncNow} disabled={busy}>{busy ? 'Checking' : 'Check mail now'}</button></div>
  {/if}
  {#if google}
    <div class="form">
      <p class="note">
        Sign in with Google and CeeVee can read replies from recruiters in your Gmail and keep your
        applications up to date. CeeVee asks for read-only access: it cannot send, change or delete
        anything. You can remove access at any time in your Google account.
      </p>
      {#if signedIn}
        <p>
          <b>Signed in with Google</b> as {signedIn.address}.
          <button class="ghost" onclick={() => disconnect(signedIn!)} disabled={busy}>Sign out of Google</button>
        </p>
      {/if}
      <div class="row">
        {#if busy}
          <span class="note">Waiting for Google. Finish in your browser, or cancel to start over.</span>
          <button class="ghost" onclick={cancelGoogle}>Cancel</button>
        {:else}
          <button onclick={signInWithGoogle}>{signedIn ? 'Sign in with a different Google account' : 'Sign in with Google'}</button>
        {/if}
      </div>
    </div>
    <p class="note">Or connect any email account with an app password:</p>
  {/if}
  <div class="form">
    <input aria-label="Email address" placeholder="Email address" bind:value={address} onblur={fillServer} />
    <div class="row">
      <input aria-label="Mail server" placeholder="Mail server, for example imap.gmail.com" bind:value={host} />
      <input aria-label="Port" type="number" class="port" bind:value={port} />
    </div>
    <input aria-label="App password" type="password" placeholder="App password" bind:value={password} autocomplete="off" />
    <p class="note">
      Use an app password, not your normal password. Gmail, Outlook, Yahoo and iCloud each let you
      create one in their security settings.
    </p>
    <div class="row">
      <button onclick={connect} disabled={busy || !looksLikeAddress(address) || host.trim() === '' || password === ''}>
        {busy ? 'Checking the sign-in' : 'Connect email'}
      </button>
    </div>
  </div>
  {#if message}<p role="status">{message}</p>{/if}
  {#if error}<p role="alert">{error}</p>{/if}
</section>

<style>
  section { display: flex; flex-direction: column; gap: 16px; max-width: 760px; margin-top: 24px; }
  .item { display: flex; justify-content: space-between; gap: 16px; align-items: center; border: 1px solid var(--line); background: var(--surface); padding: 16px; }
  .form { display: flex; flex-direction: column; gap: 8px; }
  .row { display: flex; gap: 8px; flex-wrap: wrap; }
  .row input { flex: 1; min-width: 140px; }
  .row .port { flex: 0 0 96px; min-width: 0; }
  b { color: var(--ink); font-weight: 500; }
</style>
