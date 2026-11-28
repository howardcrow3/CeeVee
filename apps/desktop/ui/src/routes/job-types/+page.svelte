<script lang="ts">
  import { onMount } from 'svelte';
  import { commands, type CategoryLine } from '$lib/bindings';
  import { describeCategory, edited, formFrom, newCategory } from '$lib/categories';
  import Toggle from '$lib/components/Toggle.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';

  let items = $state<CategoryLine[]>([]);
  let loaded = $state(false);
  let error = $state('');
  let creating = $state(false);
  let name = $state('');
  let titles = $state('');
  let keywords = $state('');
  let locations = $state('');
  let avoid = $state('');
  let remote_ok = $state(true);
  let min_pay = $state('');
  let remote_only = $state(false);

  async function load() {
    try {
      const r = await commands.listCategories();
      if (r.status === 'ok') items = r.data;
      else error = r.error;
    } catch (e) {
      error = `Running outside the desktop app: ${String(e)}`;
    } finally {
      loaded = true;
    }
  }
  onMount(load);

  async function add() {
    error = '';
    const r = await commands.createCategory(newCategory({ name, titles, keywords, locations, avoid, remote_ok, min_pay, remote_only }));
    if (r.status !== 'ok') {
      error = r.error;
      return;
    }
    name = titles = keywords = locations = avoid = '';
    remote_ok = true;
    min_pay = '';
    remote_only = false;
    creating = false;
    await load();
  }

  let editing = $state<number | null>(null);
  let form = $state(formFrom({ id: 0, name: '', titles: [], keywords: [], locations: [], remote_ok: true, avoid: [], min_pay: 0, remote_only: false, active: true }));
  let removing = $state<number | null>(null);

  function startEdit(c: CategoryLine) {
    editing = c.id;
    removing = null;
    form = formFrom(c);
  }

  async function save(c: CategoryLine) {
    error = '';
    const r = await commands.updateCategory(edited(c, form));
    if (r.status !== 'ok') {
      error = r.error;
      return;
    }
    editing = null;
    await load();
  }

  async function remove(c: CategoryLine) {
    error = '';
    const r = await commands.deleteCategory(c.id);
    removing = null;
    if (r.status === 'ok') await load();
    else error = r.error;
  }

  async function setActive(c: CategoryLine, on: boolean) {
    const r = await commands.setCategoryActive(c.id, on);
    if (r.status === 'ok') await load();
    else error = r.error;
  }
</script>

<header class="head">
  <span class="label">Job types</span>
  <button onclick={() => (creating = !creating)}>{creating ? 'Cancel' : 'New job type'}</button>
</header>
<p class="note lede">Each job type is a saved search. CeeVee sorts new postings into Strong, Possible and Skip against its titles, words and places, and says why. Only remote jobs and lowest pay are hard rules: a job that breaks one is always Skip. Separate several entries with commas.</p>

{#if creating}
  <section class="form">
    <label>Name<input placeholder="For example Product jobs" bind:value={name} /></label>
    <label>Job titles<input bind:value={titles} /></label>
    <label>Words the posting should mention<input bind:value={keywords} /></label>
    <label>Places<input bind:value={locations} /></label>
    <label>Words to avoid<input bind:value={avoid} /></label>
    <div class="line"><Toggle bind:checked={remote_ok} label="Remote jobs are fine" /><span>Remote jobs are fine</span></div>
    <div class="line"><Toggle bind:checked={remote_only} label="Only remote jobs" /><span>Only remote jobs</span></div>
    <label>Lowest yearly pay<input placeholder="For example 120k" bind:value={min_pay} /></label>
    <div class="row"><button onclick={add} disabled={name.trim() === ''}>Create</button></div>
  </section>
{/if}
{#if error}<p role="alert">{error}</p>{/if}

{#if loaded && !error && items.length === 0 && !creating}
  <EmptyState title="No job types yet">
    <p>A job type says what you are looking for, such as Product jobs in Denver. Choose New job type to add one.</p>
  </EmptyState>
{:else if items.length > 0}
  <ul class="list" aria-label="Job types">
    {#each items as c (c.id)}
      <li class:off={!c.active && editing !== c.id} class:edit={editing === c.id}>
        {#if editing === c.id}
          <section class="form" aria-label={`Edit ${c.name}`}>
            <label>Name<input bind:value={form.name} /></label>
            <label>Job titles<input bind:value={form.titles} /></label>
            <label>Words the posting should mention<input bind:value={form.keywords} /></label>
            <label>Places<input bind:value={form.locations} /></label>
            <label>Words to avoid<input bind:value={form.avoid} /></label>
            <div class="line"><Toggle bind:checked={form.remote_ok} label="Remote jobs are fine" /><span>Remote jobs are fine</span></div>
            <div class="line"><Toggle bind:checked={form.remote_only} label="Only remote jobs" /><span>Only remote jobs</span></div>
            <label>Lowest yearly pay<input placeholder="For example 120k" bind:value={form.min_pay} /></label>
            <div class="row">
              <button onclick={() => save(c)} disabled={form.name.trim() === ''}>Save</button>
              <button class="ghost" onclick={() => (editing = null)}>Cancel</button>
            </div>
          </section>
        {:else}
          <div class="text">
            <b>{c.name}</b>
            <span class="note">{describeCategory(c)}</span>
          </div>
          <div class="acts">
            {#if removing === c.id}
              <span class="note">Remove it? Jobs already found stay.</span>
              <button onclick={() => remove(c)}>Remove</button>
              <button class="ghost" onclick={() => (removing = null)}>Keep</button>
            {:else}
              <button class="ghost" onclick={() => startEdit(c)}>Edit</button>
              <button class="ghost" onclick={() => (removing = c.id)}>Remove</button>
            {/if}
            <Toggle checked={c.active} label={`${c.name} is on`} onchange={(v) => setActive(c, v)} />
          </div>
        {/if}
      </li>
    {/each}
  </ul>
{/if}

<style>
  .head { display: flex; align-items: center; justify-content: space-between; }
  .lede { max-width: 60ch; margin: 0; }
  .list { list-style: none; margin: 0; padding: 0; border: 1px solid var(--line); border-radius: var(--r-md); background: var(--surface); }
  .list li { display: flex; align-items: center; justify-content: space-between; gap: 16px; padding: 8px 12px; border-bottom: 1px solid var(--line); }
  .list li:last-child { border-bottom: 0; }
  .list li.off .text { opacity: 0.6; }
  .list li.edit { display: block; }
  .acts { display: flex; align-items: center; gap: 8px; flex-shrink: 0; }
  .text { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  b { color: var(--ink); font-weight: 500; }
  .form { display: flex; flex-direction: column; gap: 8px; max-width: 520px; padding: 12px; border: 1px solid var(--line); border-radius: var(--r-md); background: var(--surface); }
  .form label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--muted); }
  .line { display: flex; align-items: center; gap: 8px; }
  .row { display: flex; gap: 8px; }
</style>
