<script lang="ts">
  import type { Snippet } from 'svelte';
  import ChatPane from './ChatPane.svelte';
  import { chatPane, toggleChatPane } from '$lib/chat-ui.svelte';

  let { focus = '', children }: { focus?: string; children: Snippet } = $props();
</script>

<div class="layout" class:open={chatPane.open}>
  <div class="main">
    <div class="tog">
      <button class="ghost" aria-pressed={chatPane.open} onclick={toggleChatPane}>{chatPane.open ? 'Hide chat' : 'Ask CeeVee'}</button>
    </div>
    {@render children()}
  </div>
  {#if chatPane.open}
    <aside class="pane" aria-label="Chat"><ChatPane {focus} compact /></aside>
  {/if}
</div>

<style>
  .layout { display: grid; grid-template-columns: minmax(0, 1fr); gap: 16px; align-items: start; }
  .layout.open { grid-template-columns: minmax(0, 1fr) 320px; }
  .main { display: flex; flex-direction: column; gap: 12px; min-width: 0; }
  .tog { display: flex; justify-content: flex-end; }
  .pane { position: sticky; top: 0; border-left: 1px solid var(--line); padding-left: 16px; }
</style>
