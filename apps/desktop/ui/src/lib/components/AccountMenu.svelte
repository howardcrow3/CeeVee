<script lang="ts">
  import { openSettings } from '$lib/settings-ui.svelte';
  import { onMount } from 'svelte';
  import { applyTheme, loadTheme, saveTheme, type ThemeChoice } from '$lib/theme';
  import { settingsTabs } from '$lib/settings';
  import { menuLicense, pillText } from '$lib/license';
  import { licenseUi, openBuyPage, refreshLicense } from '$lib/license-ui.svelte';

  let { canFeedback = false, onfeedback }: { canFeedback?: boolean; onfeedback?: () => void } = $props();
  let open = $state(false);
  let theme = $state<ThemeChoice>('system');
  let root: HTMLDivElement | undefined = $state();

  let pill = $derived(pillText(licenseUi.view));
  let lic = $derived(menuLicense(licenseUi.view));

  function toggle() {
    open = !open;
    if (open) void refreshLicense();
  }

  onMount(() => {
    void refreshLicense();
    theme = loadTheme();
    applyTheme(theme, document.documentElement);
  });

  function setTheme(t: ThemeChoice) {
    theme = t;
    saveTheme(t);
    applyTheme(t, document.documentElement);
  }
  function onWindowClick(e: MouseEvent) {
    if (open && root && !root.contains(e.target as Node)) open = false;
  }
  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') open = false;
  }
  const themes: { value: ThemeChoice; label: string }[] = [
    { value: 'system', label: 'System' },
    { value: 'light', label: 'Light' },
    { value: 'dark', label: 'Dark' }
  ];
</script>

<svelte:window onclick={onWindowClick} onkeydown={onKey} />

<div class="account" bind:this={root}>
  {#if open}
    <div class="menu" role="menu" aria-label="Account">
      {#if lic}
        <div class="lic {lic.tone}" role="group" aria-label="License">
          <span class="lt">{lic.title}</span>
          <span class="ld">{lic.detail}</span>
          {#if lic.buy}<button class="buy" onclick={() => { open = false; void openBuyPage(); }}>Buy CeeVee</button>{/if}
        </div>
      {/if}
      <div class="grp">Settings</div>
      {#each settingsTabs as t (t.id)}
        <button role="menuitem" onclick={() => { open = false; openSettings(t.id); }}>{t.label}</button>
      {/each}
      {#if canFeedback}
        <div class="sep"></div>
        <button role="menuitem" onclick={() => { open = false; onfeedback?.(); }}>Send feedback</button>
      {/if}
      <div class="sep"></div>
      <div class="row" role="group" aria-label="Theme">
        <span class="lbl">Theme</span>
        <div class="seg">
          {#each themes as t (t.value)}
            <button class:on={theme === t.value} aria-pressed={theme === t.value} onclick={() => setTheme(t.value)}>{t.label}</button>
          {/each}
        </div>
      </div>
    </div>
  {/if}
  <button class="trigger" aria-haspopup="menu" aria-expanded={open} onclick={toggle}>
    <span class="avatar" aria-hidden="true">C</span>
    <span class="who">
      <span class="n">Account</span>
      {#if pill}<span class="s {pill.tone}">{pill.text}</span>{/if}
    </span>
  </button>
</div>

<style>
  .account { position: relative; }
  .trigger { display: flex; align-items: center; gap: 8px; width: 100%; min-height: 38px; padding: 4px 6px; border: 0; background: transparent;
    color: var(--ink); font-size: 13px; font-weight: 500; border-radius: var(--r-sm); text-align: left; }
  .trigger:hover:not(:disabled) { background: var(--hover); color: var(--ink); }
  .avatar { width: 22px; height: 22px; border-radius: var(--r-pill); background: var(--accent); color: var(--on-accent);
    display: grid; place-items: center; font-size: 11px; font-weight: 600; }
  .who { display: flex; flex-direction: column; min-width: 0; line-height: 1.25; }
  .n { font-size: 13px; font-weight: 500; }
  .s { font-size: 11px; color: var(--muted); font-weight: 400; }
  .s.soon { color: var(--ink); font-weight: 500; }
  .s.ended { color: var(--danger); font-weight: 500; }
  .menu { position: absolute; bottom: calc(100% + 6px); left: 0; right: 0; min-width: 220px; max-height: calc(100vh - 80px); overflow-y: auto; background: var(--surface); border: 1px solid var(--line);
    border-radius: var(--r-md); box-shadow: var(--shadow-pop); padding: 4px; display: flex; flex-direction: column; gap: 1px; z-index: 20; }
  .menu > button { display: block; height: 28px; line-height: 28px; padding: 0 8px; border: 0; background: transparent;
    color: var(--body); text-decoration: none; font-size: 13px; text-align: left; border-radius: var(--r-sm); }
  .menu > button:hover:not(:disabled) { background: var(--hover); color: var(--ink); }
  .lic { margin: 2px 0 4px; padding: 8px 10px; border: 1px solid var(--accent); background: var(--accent-soft); border-radius: var(--r-sm); display: flex; flex-direction: column; gap: 6px; }
  .lic.ended { border-color: var(--danger); background: transparent; }
  .lt { font-size: 12px; font-weight: 600; color: var(--ink); }
  .lic.ended .lt { color: var(--danger); }
  .ld { font-size: 11.5px; color: var(--body); }
  .grp { font-size: 11px; color: var(--muted); font-weight: 500; padding: 6px 8px 2px; }
  .buy { border: 0; border-radius: var(--r-sm); font-size: 12px; padding: 0 8px; height: 26px; line-height: 26px; text-align: center; background: var(--ink); color: var(--on-ink); font-weight: 500; }
  .sep { height: 1px; background: var(--line); margin: 4px 0; }
  .row { display: flex; align-items: center; justify-content: space-between; padding: 2px 4px 2px 8px; }
  .lbl { font-size: 12px; color: var(--muted); }
  .seg { display: inline-flex; border: 1px solid var(--line); border-radius: var(--r-sm); overflow: hidden; }
  .seg button { height: 22px; padding: 0 8px; border: 0; border-radius: 0; background: transparent; color: var(--body); font-size: 12px; }
  .seg button.on { background: var(--selected); color: var(--ink); }
  .seg button:hover:not(:disabled):not(.on) { background: var(--hover); }
</style>
