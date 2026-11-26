<script lang="ts">
  import { openSettings } from '$lib/settings-ui.svelte';
  import { onMount } from 'svelte';
  import { applyTheme, loadTheme, saveTheme, type ThemeChoice } from '$lib/theme';

  let { canFeedback = false, onfeedback }: { canFeedback?: boolean; onfeedback?: () => void } = $props();
  let open = $state(false);
  let theme = $state<ThemeChoice>('system');
  let root: HTMLDivElement | undefined = $state();

  onMount(() => {
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
      <button role="menuitem" onclick={() => { open = false; openSettings(); }}>Settings</button>
      {#if canFeedback}
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
  <button class="trigger" aria-haspopup="menu" aria-expanded={open} onclick={() => (open = !open)}>
    <span class="avatar" aria-hidden="true">C</span>
    <span class="who">Account</span>
  </button>
</div>

<style>
  .account { position: relative; margin-top: auto; }
  .trigger { display: flex; align-items: center; gap: 8px; width: 100%; height: 34px; padding: 0 6px; border: 0; background: transparent;
    color: var(--ink); font-size: 13px; font-weight: 500; border-radius: var(--r-sm); text-align: left; }
  .trigger:hover:not(:disabled) { background: var(--hover); color: var(--ink); }
  .avatar { width: 22px; height: 22px; border-radius: var(--r-pill); background: var(--accent); color: var(--on-accent);
    display: grid; place-items: center; font-size: 11px; font-weight: 600; }
  .menu { position: absolute; bottom: 40px; left: 0; width: 220px; background: var(--surface); border: 1px solid var(--line);
    border-radius: var(--r-md); box-shadow: var(--shadow-pop); padding: 4px; display: flex; flex-direction: column; gap: 1px; z-index: 20; }
  .menu > button { display: block; height: 28px; line-height: 28px; padding: 0 8px; border: 0; background: transparent;
    color: var(--body); text-decoration: none; font-size: 13px; text-align: left; border-radius: var(--r-sm); }
  .menu > button:hover:not(:disabled) { background: var(--hover); color: var(--ink); }
  .sep { height: 1px; background: var(--line); margin: 4px 0; }
  .row { display: flex; align-items: center; justify-content: space-between; padding: 2px 4px 2px 8px; }
  .lbl { font-size: 12px; color: var(--muted); }
  .seg { display: inline-flex; border: 1px solid var(--line); border-radius: var(--r-sm); overflow: hidden; }
  .seg button { height: 22px; padding: 0 8px; border: 0; border-radius: 0; background: transparent; color: var(--body); font-size: 12px; }
  .seg button.on { background: var(--selected); color: var(--ink); }
  .seg button:hover:not(:disabled):not(.on) { background: var(--hover); }
</style>
