<script lang="ts">
  import { onMount } from 'svelte';
  import { commands, type BoardLine } from '$lib/bindings';
  import { boardKinds, cleanBoards } from '$lib/boards';

  let boards = $state<BoardLine[]>([]);
  let message = $state('');
  let error = $state('');

  onMount(async () => {
    try {
      const r = await commands.listBoards();
      if (r.status === 'ok') boards = r.data;
      else error = r.error;
    } catch (e) {
      error = `Running outside the desktop app: ${String(e)}`;
    }
  });

  function add() {
    boards = [...boards, { kind: 'greenhouse', token: '', company: '' }];
  }
  function remove(i: number) {
    boards = boards.filter((_, j) => j !== i);
  }
  async function save() {
    message = '';
    error = '';
    const cleaned = cleanBoards(boards);
    const r = await commands.saveBoards(cleaned);
    if (r.status === 'ok') {
      boards = cleaned;
      message = 'Saved.';
    } else error = r.error;
  }
</script>

<span class="label">Settings</span>
<section>
  <h2>Companies to watch</h2>
  <p class="note">
    CeeVee reads the public job lists of these companies. The board name is the last part of the
    company's job page address, for example boards.greenhouse.io/<b>acme</b>.
  </p>
  {#each boards as b, i (i)}
    <div class="row">
      <select aria-label="Job board" bind:value={b.kind}>
        {#each boardKinds as k (k.value)}<option value={k.value}>{k.label}</option>{/each}
      </select>
      <input aria-label="Board name" placeholder="Board name" bind:value={b.token} />
      <input aria-label="Company" placeholder="Company" bind:value={b.company} />
      <button class="ghost" onclick={() => remove(i)}>Remove</button>
    </div>
  {/each}
  <div class="row">
    <button class="ghost" onclick={add}>Add a company</button>
    <button onclick={save}>Save</button>
  </div>
  {#if message}<p role="status">{message}</p>{/if}
  {#if error}<p role="alert">{error}</p>{/if}
</section>

<style>
  section { display: flex; flex-direction: column; gap: 16px; max-width: 760px; margin-top: 24px; }
  .row { display: flex; gap: 8px; flex-wrap: wrap; }
  .row input { flex: 1; min-width: 140px; }
  select { font: inherit; background: var(--paper); color: var(--ink); border: 1px solid var(--border-control); padding: 12px 16px; }
</style>
