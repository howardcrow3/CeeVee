<script lang="ts">
  import '../app.css';
  import Shell from '$lib/components/Shell.svelte';
  let { children } = $props();
</script>

<Shell>{@render children()}</Shell>
