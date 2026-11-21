<script lang="ts">
  import { onMount } from 'svelte';
  import { commands, type AnalyticsView } from '$lib/bindings';
  import { sentSummary, shouldAsk, statusLine } from '$lib/analytics';

  /** `ask` is the one-time card at first run; `settings` is the always-available switch. */
  let { mode }: { mode: 'ask' | 'settings' } = $props();

  let view = $state<AnalyticsView | null>(null);
  let error = $state('');

  async function load() {
    try {
      const r = await commands.analyticsStatus();
      if (r.status === 'ok') view = r.data;
    } catch {
      // Running outside the desktop app: nothing to show.
    }
  }
  async function choose(share: boolean) {
    error = '';
    const r = await commands.setAnalyticsConsent(share);
    if (r.status === 'ok') await load();
    else error = r.error;
  }

  onMount(load);
</script>

{#if mode === 'ask' && shouldAsk(view)}
  <div class="banner" role="region" aria-label="Help improve CeeVee">
    <span>
      <strong>Help improve CeeVee?</strong>
      {sentSummary} You can change this any time in Settings.
    </span>
    <span class="row">
      <button class="ghost" onclick={() => choose(false)}>No thanks</button>
      <button onclick={() => choose(true)}>Share anonymous counts</button>
    </span>
  </div>
{:else if mode === 'settings' && view}
  <section>
    <h2>Usage analytics</h2>
    <p>{sentSummary}</p>
    <p>{statusLine(view)}</p>
    {#if error}<p role="alert">{error}</p>{/if}
    {#if view.consent === 'granted'}
      <button class="ghost" onclick={() => choose(false)}>Stop sharing</button>
    {:else}
      <button onclick={() => choose(true)}>Share anonymous counts</button>
    {/if}
  </section>
{/if}

<style>
  .banner { display: flex; justify-content: space-between; gap: 16px; align-items: baseline; border: 1px solid var(--line); background: var(--surface); padding: 12px 16px; }
  .row { display: flex; gap: 8px; flex-shrink: 0; }
</style>
