<script lang="ts">
  import { page } from '$app/state';
  import { resolve } from '$app/paths';
  import { sections, isCurrent } from '$lib/nav';
  import LicenseBanner from './LicenseBanner.svelte';
  import CrashNotice from './CrashNotice.svelte';
  import AnalyticsPanel from './AnalyticsPanel.svelte';
  import FeedbackDialog from './FeedbackDialog.svelte';
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
    {#if canFeedback}
      <span class="grow"></span>
      <button class="feedback" onclick={() => (feedbackOpen = true)}>Send feedback</button>
    {/if}
  </nav>
  <main class="content"><CrashNotice /><LicenseBanner /><AnalyticsPanel mode="ask" />{@render children()}</main>
</div>
{#if canFeedback}<FeedbackDialog bind:open={feedbackOpen} />{/if}

<style>
  .shell { display: grid; grid-template-columns: 200px 1fr; min-height: 100vh; }
  .side { border-right: 1px solid var(--line); padding: 24px 16px; display: flex; flex-direction: column; gap: 8px; }
  .mark { font-size: 15px; font-weight: 600; letter-spacing: -0.02em; color: var(--ink); margin-bottom: 24px; }
  .side a { color: var(--body); text-decoration: none; padding: 8px 0; font-size: 13px;
    background: linear-gradient(var(--accent), var(--accent)) 0 100% / 0 1px no-repeat; transition: background-size 300ms var(--ease); }
  .side a:hover { background-size: 100% 1px; }
  .side a.cur { color: var(--ink); background-size: 100% 1px; }
  .grow { flex: 1; }
  .feedback { background: transparent; color: var(--body); border: 0; border-top: 1px solid var(--line); padding: 16px 0 0;
    text-align: left; font: 400 13px var(--font-text); letter-spacing: 0; text-transform: none; }
  .feedback:hover:not(:disabled) { background: transparent; color: var(--ink); }
  .content { padding: 32px; display: flex; flex-direction: column; gap: 24px; min-width: 0; }
</style>
