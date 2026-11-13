<script lang="ts">
  import { page } from '$app/state';
  import { resolve } from '$app/paths';
  import { sections, isCurrent } from '$lib/nav';
  let { children } = $props();
</script>

<div class="shell">
  <nav class="side" aria-label="Sections">
    <div class="mark">CeeVee</div>
    {#each sections as s (s.href)}
      <a href={resolve(s.href)} class:cur={isCurrent(s.href, page.url.pathname)} aria-current={isCurrent(s.href, page.url.pathname) ? 'page' : undefined}>{s.label}</a>
    {/each}
  </nav>
  <main class="content">{@render children()}</main>
</div>

<style>
  .shell { display: grid; grid-template-columns: 200px 1fr; min-height: 100vh; }
  .side { border-right: 1px solid var(--line); padding: 24px 16px; display: flex; flex-direction: column; gap: 8px; }
  .mark { font-family: var(--font-display); font-size: 24px; color: var(--ink); margin-bottom: 24px; }
  .side a { color: var(--body); text-decoration: none; padding: 8px 0; font-size: 13px;
    background: linear-gradient(var(--accent), var(--accent)) 0 100% / 0 1px no-repeat; transition: background-size 300ms var(--ease); }
  .side a:hover { background-size: 100% 1px; }
  .side a.cur { color: var(--ink); background-size: 100% 1px; }
  .content { padding: 32px; display: flex; flex-direction: column; gap: 24px; min-width: 0; }
</style>
