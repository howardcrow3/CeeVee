<script lang="ts">
  import { onMount } from 'svelte';
  import { commands, type ProgressView } from '$lib/bindings';
  import { funnelText, goalText, moveText } from '$lib/people';

  let { refresh = 0 }: { refresh?: number } = $props();

  let view = $state<ProgressView | null>(null);
  let error = $state('');
  let thanked = $state<Set<number>>(new Set());

  async function load() {
    try {
      const r = await commands.networkProgress();
      if (r.status === 'ok') view = r.data;
      else error = r.error;
    } catch (e) {
      error = String(e);
    }
  }
  onMount(load);
  $effect(() => {
    void refresh;
    load();
  });

  async function setGoal(value: string) {
    const r = await commands.setNetworkGoal(Number(value));
    if (r.status !== 'ok') error = r.error;
    await load();
  }

  async function congratulated(id: number) {
    const r = await commands.logContact(id, 'message', 'Congratulated on the new job');
    if (r.status !== 'ok') {
      error = r.error;
      return;
    }
    thanked = new Set([...thanked, id]);
    await load();
  }
</script>

{#if view}
  <section>
    <h2>This week</h2>
    <p class="line">
      {goalText(view.this_week, view.goal)}
      <label class="goal">Goal
        <select aria-label="Weekly goal" value={String(view.goal)} onchange={(e) => setGoal(e.currentTarget.value)}>
          {#each [1, 2, 3, 5, 7, 10] as n (n)}<option value={String(n)}>{n} a week</option>{/each}
        </select>
      </label>
    </p>
    {#if funnelText(view.asked, view.replied, view.referred)}
      <p class="muted">{funnelText(view.asked, view.replied, view.referred)}</p>
    {/if}
    {#if view.moves.filter((m) => !thanked.has(m.id)).length > 0}
      <h3>New jobs to congratulate</h3>
      <ul>
        {#each view.moves.filter((m) => !thanked.has(m.id)) as m (m.id)}
          <li>
            <div class="who">
              <b>{m.name}</b>
              <span class="muted">{moveText(m.from_company, m.to_company, m.to_title)}</span>
            </div>
            <button class="ghost" onclick={() => congratulated(m.id)}>I congratulated them</button>
          </li>
        {/each}
      </ul>
    {/if}
    {#if error}<p role="alert">{error}</p>{/if}
  </section>
{/if}

<style>
  section { margin-top: 24px; max-width: 760px; }
  h2 { margin: 0 0 8px; }
  h3 { margin: 16px 0 8px; font-size: 13px; }
  p { margin: 0 0 4px; }
  .line { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
  .goal { display: inline-flex; align-items: center; gap: 6px; }
  .muted { opacity: 0.8; }
  ul { list-style: none; padding: 0; margin: 0; border: 1px solid var(--line); background: var(--surface); }
  li { display: flex; justify-content: space-between; align-items: center; gap: 12px; padding: 8px 12px; border-bottom: 1px solid var(--line); }
  li:last-child { border-bottom: none; }
  .who { display: flex; flex-direction: column; min-width: 0; }
  b { color: var(--ink); font-weight: 500; }
  select { font: inherit; background: var(--paper); color: var(--ink); border: 1px solid var(--border-control); padding: 4px 8px; }
</style>
