<script lang="ts">
  import { emptyJob, emptySchool, parseResume, serializeResume, type Resume } from '$lib/resume';

  let { value = $bindable() }: { value: string } = $props();

  let r = $state<Resume | null>(parseResume(value));
  let last = value;

  // Pick up changes from outside (loading another document, undo, chat edits).
  $effect(() => {
    if (value !== last) {
      last = value;
      r = parseResume(value);
    }
  });

  function push() {
    if (!r) return;
    last = serializeResume(r, value);
    value = last;
  }
</script>

{#if r}
  <div class="form" oninput={push} onchange={push} role="group">
    <div class="grid">
      <label>Name<input bind:value={r.name} /></label>
      <label>Headline<input bind:value={r.label} /></label>
      <label>Email<input type="email" bind:value={r.email} /></label>
      <label>Phone<input bind:value={r.phone} /></label>
      <label>City<input bind:value={r.location} /></label>
    </div>
    <label>Summary<textarea rows="3" bind:value={r.summary}></textarea></label>

    <span class="label">Work</span>
    {#each r.work as job, i (i)}
      <fieldset>
        <div class="grid">
          <label>Company<input bind:value={job.name} /></label>
          <label>Title<input bind:value={job.position} /></label>
          <label>Start<input placeholder="2021-03" bind:value={job.startDate} /></label>
          <label>End<input placeholder="Present" bind:value={job.endDate} /></label>
        </div>
        {#each job.highlights.keys() as h (h)}
          <label>Bullet<input bind:value={job.highlights[h]} /></label>
        {/each}
        <div class="row">
          <button class="ghost" onclick={() => { job.highlights.push(''); push(); }}>Add bullet</button>
          <button class="ghost" onclick={() => { r?.work.splice(i, 1); push(); }}>Remove job</button>
        </div>
      </fieldset>
    {/each}
    <button class="ghost" onclick={() => { r?.work.push(emptyJob()); push(); }}>Add job</button>

    <span class="label">Education</span>
    {#each r.education as ed, i (i)}
      <fieldset>
        <div class="grid">
          <label>School<input bind:value={ed.institution} /></label>
          <label>Degree<input bind:value={ed.studyType} /></label>
          <label>Field<input bind:value={ed.area} /></label>
          <label>Finished<input bind:value={ed.endDate} /></label>
        </div>
        <button class="ghost" onclick={() => { r?.education.splice(i, 1); push(); }}>Remove</button>
      </fieldset>
    {/each}
    <button class="ghost" onclick={() => { r?.education.push(emptySchool()); push(); }}>Add school</button>

    <span class="label">Skills</span>
    <label>One per line
      <textarea
        rows="4"
        value={r.skills.join('\n')}
        oninput={(e) => { if (r) { r.skills = e.currentTarget.value.split('\n'); push(); } }}
      ></textarea>
    </label>
  </div>
{:else}
  <p role="alert">This resume is not valid JSON, so the form cannot open it. Use "Edit as text" to fix it.</p>
{/if}

<style>
  .form { display: flex; flex-direction: column; gap: 16px; }
  .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 12px; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; letter-spacing: 0.1em; text-transform: uppercase; color: var(--body); }
  input, textarea { font: 400 14px/1.5 var(--font-body, inherit); background: var(--paper); color: var(--ink); border: 1px solid var(--border-control); padding: 8px 12px; text-transform: none; letter-spacing: 0; width: 100%; box-sizing: border-box; }
  fieldset { border: 1px solid var(--border-control); padding: 16px; margin: 0; display: flex; flex-direction: column; gap: 12px; min-width: 0; }
  .row { display: flex; gap: 8px; flex-wrap: wrap; }
  p { margin: 0; }
</style>
