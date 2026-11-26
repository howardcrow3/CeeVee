<script lang="ts">
  import { openSettings } from '$lib/settings-ui.svelte';
  import { resolve } from '$app/paths';
  import { commands, type ProfileChange } from '$lib/bindings';
  import { accepted, savedSummary } from '$lib/onboarding';
  import GoogleSignIn from '$lib/components/GoogleSignIn.svelte';

  let text = $state('');
  let changes = $state<ProfileChange[]>([]);
  let ticked = $state<boolean[]>([]);
  let reading = $state(false);
  let error = $state('');
  let message = $state('');

  async function read() {
    error = '';
    message = '';
    reading = true;
    try {
      const r = await commands.previewResumeImport(text);
      if (r.status === 'ok') {
        changes = r.data;
        ticked = r.data.map(() => true);
        if (r.data.length === 0) message = 'Your profile already has everything in this resume.';
      } else error = r.error;
    } catch (e) {
      error = String(e);
    } finally {
      reading = false;
    }
  }

  async function save() {
    error = '';
    const r = await commands.applyProfileChanges(accepted(changes, ticked));
    if (r.status === 'ok') {
      message = savedSummary(r.data);
      changes = [];
      ticked = [];
    } else error = r.error;
  }
</script>

<span class="label">Welcome</span>
<h1>Set up CeeVee</h1>
<ol>
  <li>
    <h2>1. Download the local AI</h2>
    <p>CeeVee reads your resume with an AI model that runs on this computer. <button class="link" onclick={() => openSettings('general')}>Download it in Settings</button>.</p>
  </li>
  <li>
    <h2>2. Add your resume</h2>
    <p class="note">Paste the text of your resume. CeeVee shows what it found, and you choose what goes into your profile.</p>
    <textarea aria-label="Resume text" rows="10" bind:value={text}></textarea>
    <div class="row"><button onclick={read} disabled={reading || text.trim() === ''}>{reading ? 'Reading' : 'Read my resume'}</button></div>
    {#if changes.length}
      <ul class="plain">
        {#each changes as c, i (i)}
          <li><label><input type="checkbox" bind:checked={ticked[i]} /> {c.text}</label></li>
        {/each}
      </ul>
      <div class="row"><button onclick={save}>Save the ticked changes</button></div>
    {/if}
    {#if message}<p role="status">{message}</p>{/if}
    {#if error}<p role="alert">{error}</p>{/if}
  </li>
  <li>
    <h2>3. Add your LinkedIn connections</h2>
    <p>Optional. <a href={resolve('/network')}>Choose Connections.csv</a> to see who you know at each company.</p>
  </li>
  <li>
    <h2>4. Say what jobs you want</h2>
    <p>Add a <a href={resolve('/job-types')}>job type</a> for the work you want, and the companies to watch in <button class="link" onclick={() => openSettings('search')}>Settings</button>.</p>
  </li>
  <li>
    <h2>5. Connect your email</h2>
    <GoogleSignIn />
    <p class="note">Prefer not to use Google? Add any email account in <button class="link" onclick={() => openSettings('accounts')}>Settings</button>.</p>
  </li>
</ol>

<style>
  ol { display: flex; flex-direction: column; gap: 24px; max-width: 760px; padding: 0; list-style: none; }
  li { display: flex; flex-direction: column; gap: 12px; }
  textarea { font: inherit; background: var(--paper); color: var(--ink); border: 1px solid var(--border-control); padding: 12px 16px; }
  .row { display: flex; gap: 8px; }
  ul.plain { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 8px; }
</style>
