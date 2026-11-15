<script lang="ts">
  import { onMount } from 'svelte';
  import { commands, type CategoryLine } from '$lib/bindings';
  import { describeCategory, newCategory } from '$lib/categories';

  let items = $state<CategoryLine[]>([]);
  let name = $state('');
  let titles = $state('');
  let keywords = $state('');
  let locations = $state('');
  let avoid = $state('');
  let remote_ok = $state(true);
  let error = $state('');

  async function load() {
    const r = await commands.listCategories();
    if (r.status === 'ok') items = r.data;
    else error = r.error;
  }

  onMount(() => {
    load().catch((e) => (error = String(e)));
  });

  async function add() {
    error = '';
    const r = await commands.createCategory(
      newCategory({ name, titles, keywords, locations, avoid, remote_ok })
    );
    if (r.status !== 'ok') {
      error = r.error;
      return;
    }
    name = titles = keywords = locations = avoid = '';
    await load();
  }

  async function toggle(c: CategoryLine) {
    const r = await commands.setCategoryActive(c.id, !c.active);
    if (r.status === 'ok') await load();
    else error = r.error;
  }
</script>

<section>
  <h2>Kinds of jobs you want</h2>
  <p class="note">
    Each kind of job scores new postings from 0 to 100. Separate several entries with commas.
  </p>
  {#each items as c (c.id)}
    <div class="item">
      <div>
        <b>{c.name}</b>{c.active ? '' : ' (off)'}
        <div class="note">{describeCategory(c)}</div>
      </div>
      <button class="ghost" onclick={() => toggle(c)}>{c.active ? 'Turn off' : 'Turn on'}</button>
    </div>
  {/each}
  <div class="form">
    <input aria-label="Name" placeholder="Name, for example Product jobs" bind:value={name} />
    <input aria-label="Job titles" placeholder="Job titles" bind:value={titles} />
    <input aria-label="Words in the posting" placeholder="Words the posting should mention" bind:value={keywords} />
    <input aria-label="Places" placeholder="Places" bind:value={locations} />
    <input aria-label="Words to avoid" placeholder="Words to avoid" bind:value={avoid} />
    <label><input type="checkbox" bind:checked={remote_ok} /> Remote jobs are fine</label>
    <div class="row"><button onclick={add} disabled={name.trim() === ''}>Add</button></div>
  </div>
  {#if error}<p role="alert">{error}</p>{/if}
</section>

<style>
  section { display: flex; flex-direction: column; gap: 16px; max-width: 760px; margin-top: 24px; }
  .item { display: flex; justify-content: space-between; gap: 16px; align-items: center; }
  .form { display: flex; flex-direction: column; gap: 8px; }
  .row { display: flex; gap: 8px; }
</style>
