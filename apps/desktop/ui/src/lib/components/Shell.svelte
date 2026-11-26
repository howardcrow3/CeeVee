<script lang="ts">
  import { page } from '$app/state';
  import { resolve } from '$app/paths';
  import { sections, isCurrent } from '$lib/nav';
  import LicenseBanner from './LicenseBanner.svelte';
  import CrashNotice from './CrashNotice.svelte';
  import AnalyticsPanel from './AnalyticsPanel.svelte';
  import FeedbackDialog from './FeedbackDialog.svelte';
  import AccountMenu from './AccountMenu.svelte';
  import SettingsModal from './SettingsModal.svelte';
  import { onMount } from 'svelte';
  import { commands } from '$lib/bindings';
  let { children } = $props();
  let canFeedback = $state(false);
  let feedbackOpen = $state(false);
  onMount(async () => {
    try {
      canFeedback = await commands.feedbackAvailable();
    } catch {
      // Running outside the desktop app.
    }
  });
</script>

<div class="shell">
  <nav class="side" aria-label="Sections">
    <div class="mark">CeeVee</div>
    {#each sections as s (s.href)}
      <a href={resolve(s.href)} class:cur={isCurrent(s.href, page.url.pathname)} aria-current={isCurrent(s.href, page.url.pathname) ? 'page' : undefined}>{s.label}</a>
    {/each}
    <AccountMenu {canFeedback} onfeedback={() => (feedbackOpen = true)} />
  </nav>
  <main class="content"><CrashNotice /><LicenseBanner /><AnalyticsPanel mode="ask" />{@render children()}</main>
</div>
<SettingsModal />
{#if canFeedback}<FeedbackDialog bind:open={feedbackOpen} />{/if}

<style>
  .shell { display: grid; grid-template-columns: 208px 1fr; min-height: 100vh; }
  .side { background: var(--sidebar); border-right: 1px solid var(--line); padding: 12px 8px; display: flex; flex-direction: column; gap: 1px; }
  .mark { font-size: 15px; font-weight: 600; letter-spacing: -0.02em; color: var(--ink); padding: 4px 8px 12px; }
  .side a { display: flex; align-items: center; height: 28px; padding: 0 8px; border-radius: var(--r-sm); color: var(--body); text-decoration: none; font-size: 13px; }
  .side a:hover { background: var(--hover); color: var(--ink); }
  .side a.cur { background: var(--selected); color: var(--ink); font-weight: 500; }
  .content { padding: 24px 32px; display: flex; flex-direction: column; gap: 20px; min-width: 0; }
</style>
