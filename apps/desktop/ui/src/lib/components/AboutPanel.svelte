<script lang="ts">
  import { onMount } from 'svelte';
  import { commands, type Diagnostics } from '$lib/bindings';

  let info = $state<Diagnostics | null>(null);
  let busy = $state(false);
  let message = $state('');
  let error = $state('');
  let found = $state<string | null>(null);

  async function check() {
    busy = true;
    message = '';
    error = '';
    found = null;
    try {
      const r = await commands.checkForUpdate();
      if (r.status !== 'ok') error = 'Could not check for updates.';
      else if (r.data) found = r.data.version;
      else message = 'You have the latest version.';
    } catch {
      error = 'Could not check for updates.';
    }
    busy = false;
  }

  async function install() {
    busy = true;
    error = '';
    message = 'Downloading the update. CeeVee restarts when it is ready.';
    const r = await commands.installUpdate();
    if (r.status !== 'ok') {
      message = '';
      error = `The update did not install: ${r.error}`;
    }
    busy = false;
  }

  async function openPage(which: 'help' | 'privacy' | 'terms') {
    const r = await commands.openInfoPage(which);
    if (r.status !== 'ok') error = r.error;
  }

  async function openLogs() {
    const r = await commands.openLogFolder();
    if (r.status !== 'ok') error = r.error;
  }

  onMount(async () => {
    try {
      const r = await commands.diagnostics();
      if (r.status === 'ok') info = r.data;
    } catch {
      // Running outside the desktop app.
    }
  });
</script>

<section>
  <h2>About CeeVee</h2>
  {#if info}<p class="note">Version {info.version} on {info.system}.</p>{/if}
  <div class="row">
    {#if found}
      <button onclick={install} disabled={busy}>Install version {found}</button>
    {:else}
      <button class="ghost" onclick={check} disabled={busy}>{busy ? 'Checking' : 'Check for updates'}</button>
    {/if}
    <button class="ghost" onclick={openLogs}>Open log folder</button>
  </div>
  <div class="row">
    <button class="ghost" onclick={() => openPage('help')}>Help</button>
    <button class="ghost" onclick={() => openPage('privacy')}>Privacy notice</button>
    <button class="ghost" onclick={() => openPage('terms')}>Terms of use</button>
  </div>
  <p class="note">
    CeeVee keeps a small log on this computer to help fix problems. It holds no resume, job or email
    text, and it is never sent anywhere unless you send it to support yourself.
  </p>
  {#if message}<p role="status">{message}</p>{/if}
  {#if error}<p role="alert">{error}</p>{/if}
</section>

<style>
  section { display: flex; flex-direction: column; gap: 16px; max-width: 760px; margin-top: 24px; }
  .row { display: flex; gap: 8px; flex-wrap: wrap; }
</style>
