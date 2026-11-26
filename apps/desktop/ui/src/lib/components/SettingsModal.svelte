<script lang="ts">
  import { settingsTabs, type SettingsTab } from '$lib/settings';
  import { settingsUi, closeSettings } from '$lib/settings-ui.svelte';
  import LicensePanel from './LicensePanel.svelte';
  import LocalAiPanel from './LocalAiPanel.svelte';
  import CategoriesPanel from './CategoriesPanel.svelte';
  import BoardsPanel from './BoardsPanel.svelte';
  import MailAccountPanel from './MailAccountPanel.svelte';
  import AnalyticsPanel from './AnalyticsPanel.svelte';
  import AboutPanel from './AboutPanel.svelte';

  let dialog: HTMLDivElement | undefined = $state();
  $effect(() => {
    if (settingsUi.open) dialog?.focus();
  });
  function pick(id: SettingsTab) {
    settingsUi.tab = id;
  }
  function onKey(e: KeyboardEvent) {
    if (settingsUi.open && e.key === 'Escape') closeSettings();
  }
</script>

<svelte:window onkeydown={onKey} />

{#if settingsUi.open}
  <div class="scrim" role="presentation" onclick={closeSettings}></div>
  <div class="modal" role="dialog" aria-modal="true" aria-label="Settings" tabindex="-1" bind:this={dialog}>
    <nav class="tabs" aria-label="Settings sections">
      <h2>Settings</h2>
      {#each settingsTabs as t (t.id)}
        <button class:on={settingsUi.tab === t.id} aria-current={settingsUi.tab === t.id ? 'page' : undefined} onclick={() => pick(t.id)}>{t.label}</button>
      {/each}
    </nav>
    <div class="pane">
      <header>
        <h2>{settingsTabs.find((t) => t.id === settingsUi.tab)?.label}</h2>
        <button class="ghost close" aria-label="Close settings" onclick={closeSettings}>Close</button>
      </header>
      <div class="body">
        {#if settingsUi.tab === 'general'}
          <LicensePanel />
          <LocalAiPanel />
        {:else if settingsUi.tab === 'search'}
          <CategoriesPanel />
          <BoardsPanel />
        {:else if settingsUi.tab === 'accounts'}
          <MailAccountPanel />
        {:else if settingsUi.tab === 'privacy'}
          <AnalyticsPanel mode="settings" />
          <AboutPanel />
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .scrim { position: fixed; inset: 0; background: var(--scrim); z-index: 30; }
  .modal { position: fixed; z-index: 31; top: 50%; left: 50%; transform: translate(-50%, -50%); width: min(860px, calc(100vw - 48px));
    height: min(620px, calc(100vh - 48px)); display: grid; grid-template-columns: 180px 1fr; background: var(--surface);
    border: 1px solid var(--line); border-radius: var(--r-lg); box-shadow: var(--shadow-pop); overflow: hidden; outline: none; }
  .tabs { background: var(--sidebar); border-right: 1px solid var(--line); padding: 12px 8px; display: flex; flex-direction: column; gap: 1px; }
  .tabs h2 { font-size: 15px; padding: 4px 8px 12px; margin: 0; }
  .tabs button { height: 28px; padding: 0 8px; border: 0; background: transparent; color: var(--body); text-align: left;
    font-size: 13px; border-radius: var(--r-sm); }
  .tabs button:hover:not(:disabled) { background: var(--hover); color: var(--ink); }
  .tabs button.on { background: var(--selected); color: var(--ink); font-weight: 500; }
  .pane { display: flex; flex-direction: column; min-width: 0; min-height: 0; }
  header { display: flex; align-items: center; justify-content: space-between; padding: 12px 16px; border-bottom: 1px solid var(--line); }
  header h2 { margin: 0; font-size: 15px; }
  .body { padding: 16px; overflow-y: auto; display: flex; flex-direction: column; gap: 24px; }
</style>
