<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { commands } from '$lib/bindings';
  import { canActivate, keyFromInput, statusText, type LicenseView } from '$lib/license';

  let view = $state<LicenseView | null>(null);
  let input = $state('');
  let busy = $state(false);
  let message = $state('');
  let error = $state('');

  async function refresh() {
    const r = await commands.licenseStatus();
    if (r.status === 'ok') view = r.data;
    else error = r.error;
  }

  async function activate() {
    busy = true;
    message = '';
    error = '';
    const r = await commands.activateLicense(keyFromInput(input));
    if (r.status === 'ok') {
      view = r.data;
      input = '';
      message = 'Activated. Thank you!';
    } else error = r.error;
    busy = false;
  }

  async function deactivate() {
    busy = true;
    message = '';
    error = '';
    const r = await commands.deactivateLicense();
    if (r.status === 'ok') {
      view = r.data;
      message = 'This computer was removed from your license. You can use the seat on another computer.';
    } else error = r.error;
    busy = false;
  }

  async function open(which: 'buy' | 'manage') {
    const r = await commands.openLicensePage(which);
    if (r.status !== 'ok') error = r.error;
  }

  let unlisten: UnlistenFn | undefined;
  onDestroy(() => unlisten?.());

  onMount(async () => {
    try {
      // A ceevee://activate link from the purchase email was opened.
      unlisten = await listen<{ ok: boolean; message: string }>('license-link', async (e) => {
        message = e.payload.ok ? e.payload.message : '';
        error = e.payload.ok ? '' : e.payload.message;
        await refresh();
      });
      await refresh();
    } catch (e) {
      error = `Running outside the desktop app: ${String(e)}`;
    }
  });
</script>

<section>
  <h2>License</h2>
  {#if view}
    <p><span class="dot" class:on={view.state !== 'expired'} class:warn={view.state === 'expired'}></span>{statusText(view)}</p>
    {#if view.note}<p role="alert">{view.note}</p>{/if}
  {/if}
  {#if view?.state !== 'licensed'}
    <p class="note">
      CeeVee costs $19.99 once, with no subscription, and one license works on 2 of your computers.
      The key arrives by email after you buy.
    </p>
    <div class="row">
      <button onclick={() => open('buy')}>Buy CeeVee</button>
    </div>
    <div class="row">
      <input aria-label="License key" placeholder="CEEV-XXXXX-XXXXX-XXXXX-XXXXX" bind:value={input} />
      <button class="ghost" onclick={activate} disabled={busy || !canActivate(input)}>{busy ? 'Checking' : 'Activate'}</button>
    </div>
  {:else}
    <div class="row">
      <button class="ghost" onclick={() => open('manage')}>Manage my license</button>
      <button class="ghost" onclick={deactivate} disabled={busy}>Remove from this computer</button>
    </div>
  {/if}
  {#if message}<p role="status">{message}</p>{/if}
  {#if error}<p role="alert">{error}</p>{/if}
</section>

<style>
  section { display: flex; flex-direction: column; gap: 16px; max-width: 760px; margin-top: 24px; }
  .row { display: flex; gap: 8px; flex-wrap: wrap; }
  .row input { flex: 1; min-width: 240px; }
</style>
