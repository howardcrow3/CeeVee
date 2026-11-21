<script lang="ts">
  let { title, children }: { title: string; children?: import('svelte').Snippet } = $props();
</script>

<section class="empty">
  <h2>{title}</h2>
  {#if children}{@render children()}{/if}
</section>

<style>
  .empty { border: 1px solid var(--line); background: var(--surface); padding: 32px; display: flex; flex-direction: column; gap: 16px; }
</style>
