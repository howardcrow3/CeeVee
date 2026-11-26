<script lang="ts">
  import { openSettings } from '$lib/settings-ui.svelte';
  import { onMount } from 'svelte';
  import { commands } from '$lib/bindings';
  import { bannerText, type LicenseView } from '$lib/license';

  let view = $state<LicenseView | null>(null);
  let text = $derived(view ? bannerText(view) : null);

  async function buy() {
    try {
      await commands.openLicensePage('buy');
    } catch {
      // Running outside the desktop app.
    }
  }

  onMount(async () => {
    try {
      const r = await commands.licenseStatus();
      if (r.status === 'ok') view = r.data;
    } catch {
      // Running outside the desktop app: no banner.
    }
  });
</script>

{#if text}
  <div class="banner" class:expired={view?.state === 'expired'} role="status">
    <span>{text}</span>
    <span class="actions">
      <button onclick={buy}>Buy CeeVee</button>
      <button class="link" onclick={() => openSettings('general')}>License</button>
    </span>
  </div>
{/if}

<style>
  .banner { display: flex; justify-content: space-between; gap: 16px; align-items: baseline; border: 1px solid var(--line); background: var(--surface); padding: 12px 16px; }
  .banner.expired { border-color: var(--danger); }
  .actions { display: flex; gap: 12px; align-items: baseline; }
</style>
