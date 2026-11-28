<script lang="ts">
  let { checked = $bindable(false), label, onchange }: { checked?: boolean; label: string; onchange?: (v: boolean) => void } = $props();
</script>

<button type="button" class="toggle" role="switch" aria-checked={checked} aria-label={label} class:on={checked}
  onclick={() => { checked = !checked; onchange?.(checked); }}><span></span></button>

<style>
  .toggle { width: 26px; height: 15px; min-height: 0; padding: 0; border-radius: var(--r-pill); border: 1px solid var(--border-control);
    background: var(--selected); position: relative; flex: none; }
  .toggle span { position: absolute; top: 1px; left: 1px; width: 11px; height: 11px; border-radius: var(--r-pill); background: var(--ink); transition: transform 150ms ease-out; }
  .toggle.on { background: var(--accent); border-color: var(--accent); }
  .toggle.on span { transform: translateX(11px); background: var(--on-accent); }
  .toggle:hover:not(:disabled) { background: var(--hover); }
  .toggle.on:hover:not(:disabled) { background: var(--accent); }
  @media (prefers-reduced-motion: reduce) { .toggle span { transition: none; } }
</style>
