<script lang="ts">
  import { onMount } from 'svelte';
  import { commands, type ExtensionView } from '$lib/bindings';

  let view = $state<ExtensionView | null>(null);
  let error = $state('');

  async function load() {
    try {
      view = await commands.extensionStatus();
    } catch {
      // Running outside the desktop app: nothing to show.
    }
  }
  async function approve() {
    if (!view) return;
    error = '';
    const r = await commands.extensionApprove(view.pending_id);
    if (r.status === 'error') error = r.error;
    await load();
  }
  async function deny() {
    if (!view) return;
    await commands.extensionDeny(view.pending_id);
    await load();
  }
  async function disconnect() {
    await commands.extensionDisconnectAll();
    await load();
  }

  onMount(() => {
    load();
    // eslint-disable-next-line no-undef
    const t = setInterval(load, 2000);
    // eslint-disable-next-line no-undef
    return () => clearInterval(t);
  });
</script>

{#if view}
  <section>
    <h2>Browser extension</h2>
    <p>
      The CeeVee extension fills in application forms in your own browser from your profile and
      resume. It never presses Submit, and it only talks to CeeVee on this computer.
    </p>
    {#if !view.running}
      <p>The connection to the extension is not running.</p>
    {:else}
      <p>
        {view.connected === 0
          ? 'No browser is connected.'
          : view.connected === 1
            ? '1 browser is connected.'
            : `${view.connected} browsers are connected.`}
      </p>
      {#if view.pending_id}
        <div class="ask" role="alert">
          <span><strong>{view.pending_browser || 'A browser'}</strong> wants to connect to CeeVee.</span>
          <span class="row">
            <button class="ghost" onclick={deny}>Not now</button>
            <button onclick={approve}>Connect</button>
          </span>
        </div>
      {/if}
      {#if error}<p role="alert">{error}</p>{/if}
      {#if view.connected > 0}
        <button class="ghost" onclick={disconnect}>Disconnect all browsers</button>
      {/if}
    {/if}
  </section>
{/if}

<style>
  .ask { display: flex; justify-content: space-between; gap: 16px; align-items: baseline; border: 1px solid var(--line); background: var(--surface); padding: 12px 16px; margin: 8px 0; }
  .row { display: flex; gap: 8px; flex-shrink: 0; }
</style>
