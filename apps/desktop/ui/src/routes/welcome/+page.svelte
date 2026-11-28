<script lang="ts">
  import { onMount } from 'svelte';
  import { resolve } from '$app/paths';
  import { commands, type FoundJob, type MailAccountLine, type ProfileChange } from '$lib/bindings';
  import { googleAccount, foundTitle, foundStatusWord } from '$lib/inbox';
  import {
    accepted, savedSummary, stepStates, progressWord, companiesOf, foundCelebration, resumeFeedbackPrompt,
    markOnboardingFinished, type StepId
  } from '$lib/onboarding';
  import { chatPane, toggleChatPane } from '$lib/chat-ui.svelte';
  import { friendlyError } from '$lib/chat';
  import ModelSetup from '$lib/components/ModelSetup.svelte';
  import GoogleSignIn from '$lib/components/GoogleSignIn.svelte';

  let accounts = $state<MailAccountLine[]>([]);
  let modelInstalled = $state(false);
  let resumeSaved = $state(false);

  // Step 1 payoff
  let found = $state<FoundJob[]>([]);
  let scanning = $state(false);
  let scanned = $state(false);
  let scanError = $state('');

  // Step 2 payoff
  let chatOn = $state(false);

  // Step 3
  let text = $state('');
  let changes = $state<ProfileChange[]>([]);
  let ticked = $state<boolean[]>([]);
  let reading = $state(false);
  let resumeMsg = $state('');
  let resumeError = $state('');
  let feedback = $state('');
  let reviewing = $state(false);
  let feedbackError = $state('');
  let needsModel = $state(false);

  const done = $derived<Record<StepId, boolean>>({
    gmail: googleAccount(accounts) !== undefined,
    model: modelInstalled,
    resume: resumeSaved
  });
  const steps = $derived(stepStates(done));
  const stateOf = (id: StepId) => steps.find((s) => s.id === id)?.state ?? 'todo';
  const allDone = $derived(done.gmail && done.model && done.resume);

  async function loadAccounts() {
    const r = await commands.listMailAccounts();
    if (r.status === 'ok') accounts = r.data;
  }

  async function loadFound() {
    const r = await commands.listFoundJobs();
    if (r.status === 'ok') found = r.data;
  }

  /** After sign-in: read recent mail, then list the jobs it turned up. */
  async function scan() {
    scanning = true;
    scanError = '';
    try {
      const r = await commands.syncMail();
      if (r.status !== 'ok') scanError = r.error;
      await loadFound();
      scanned = true;
    } catch (e) {
      scanError = String(e);
    } finally {
      scanning = false;
    }
  }

  async function onSignedIn() {
    await loadAccounts();
    if (done.gmail && !scanned && !scanning) await scan();
  }

  async function decide(j: FoundJob, keep: boolean) {
    const r = keep ? await commands.confirmFoundJob(j.application_id) : await commands.dismissFoundJob(j.application_id);
    if (r.status === 'ok') found = found.filter((f) => f.application_id !== j.application_id);
    else scanError = r.error;
  }

  function modelReady() {
    modelInstalled = true;
  }

  function turnOnChat() {
    if (!chatPane.open) toggleChatPane();
    chatOn = true;
  }

  function onFile(e: { currentTarget: { files: ArrayLike<{ text(): Promise<string> }> | null } }) {
    const file = e.currentTarget.files?.[0];
    if (!file) return;
    file.text().then((t) => (text = t)).catch(() => (resumeError = 'CeeVee could not read that file. Paste the text of your resume instead.'));
  }

  async function read() {
    resumeError = resumeMsg = '';
    reading = true;
    try {
      const r = await commands.previewResumeImport(text);
      if (r.status === 'ok') {
        changes = r.data;
        ticked = r.data.map(() => true);
        if (r.data.length === 0) {
          resumeMsg = 'Your profile already has everything in this resume.';
          resumeSaved = true;
          await review();
        }
      } else resumeError = r.error;
    } catch (e) {
      resumeError = String(e);
    } finally {
      reading = false;
    }
  }

  async function save() {
    resumeError = '';
    const r = await commands.applyProfileChanges(accepted(changes, ticked));
    if (r.status === 'ok') {
      resumeMsg = savedSummary(r.data);
      changes = [];
      ticked = [];
      resumeSaved = true;
      await review();
    } else resumeError = r.error;
  }

  /** The step 3 payoff: ask the local AI how to make the resume better. */
  async function review() {
    reviewing = true;
    feedbackError = '';
    needsModel = false;
    try {
      const r = await commands.askModel(resumeFeedbackPrompt(text));
      if (r.status === 'ok') feedback = r.data.trim();
      else {
        const f = friendlyError(r.error);
        feedbackError = f.text;
        needsModel = f.needsModel ?? false;
      }
    } catch (e) {
      feedbackError = String(e);
    } finally {
      reviewing = false;
    }
  }

  function finish() {
    markOnboardingFinished();
  }

  onMount(async () => {
    await loadAccounts();
    if (done.gmail) await loadFound();
  });
</script>

<span class="label">Welcome</span>
<h1>Get CeeVee working for you</h1>
<p class="note">Three quick steps. Each one gives you something back right away. {progressWord(done)}.</p>

<ol class="steps">
  <li class="step" data-state={stateOf('gmail')}>
    <header><span class="num">{done.gmail ? '✓' : '1'}</span><h2>Sign in with Gmail</h2></header>
    <p>CeeVee reads your recent mail for recruiter and employer messages, and fills in the jobs you are already working on. Read-only: it never sends, changes or deletes anything.</p>
    <GoogleSignIn onchange={onSignedIn} />
    <p class="note">Google may say CeeVee is not verified yet while its review finishes. Choose Advanced, then Continue to CeeVee. You can also add any email account in Settings.</p>
    {#if scanning}<p role="status">Looking through your mail for jobs…</p>{/if}
    {#if scanned || found.length}
      <div class="payoff" role="status">
        <strong>🎉 {foundCelebration(found)}</strong>
        {#if companiesOf(found).length}
          <div class="chips">{#each companiesOf(found) as c (c)}<span class="chip">{c}</span>{/each}</div>
        {/if}
        {#if found.length}
          <ul class="plain">
            {#each found as j (j.application_id)}
              <li>
                <span><b>{foundTitle(j)}</b> <span class="note">{foundStatusWord(j.status)}</span></span>
                <span class="row">
                  <button onclick={() => decide(j, true)}>Keep</button>
                  <button class="ghost" onclick={() => decide(j, false)}>Not mine</button>
                </span>
              </li>
            {/each}
          </ul>
          <p class="note">Kept jobs go on your <a href={resolve('/applications')}>Applications</a> board.</p>
        {/if}
      </div>
    {/if}
    {#if scanError}<p role="alert">{scanError}</p>{/if}
  </li>

  <li class="step" data-state={stateOf('model')}>
    <header><span class="num">{done.model ? '✓' : '2'}</span><h2>Download the local AI</h2></header>
    <p>CeeVee's AI runs on this computer, so nothing you write leaves it. It downloads once, and you can keep using CeeVee while it does.</p>
    <ModelSetup onready={modelReady} />
    {#if done.model}
      <div class="payoff" role="status">
        <strong>🎉 The local AI is ready.</strong>
        {#if chatOn || chatPane.open}
          <p>Chat is on. Open any page and ask CeeVee about your jobs and applications.</p>
        {:else}
          <p>Switch on the chat window to talk to it from any page.</p>
          <div class="row"><button onclick={turnOnChat}>Turn on chat</button></div>
        {/if}
      </div>
    {/if}
  </li>

  <li class="step" data-state={stateOf('resume')}>
    <header><span class="num">{done.resume ? '✓' : '3'}</span><h2>Add your resume</h2></header>
    <p>Choose a text file or paste your resume. CeeVee shows what it found and you pick what goes into your profile, then the AI gives you feedback on it.</p>
    {#if !done.model}<p class="note">Reading your resume needs the local AI from step 2.</p>{/if}
    <input type="file" accept=".txt,.md,text/plain,text/markdown" aria-label="Choose a resume file" onchange={onFile} />
    <textarea aria-label="Resume text" rows="8" bind:value={text}></textarea>
    <div class="row"><button onclick={read} disabled={reading || text.trim() === ''}>{reading ? 'Reading' : 'Read my resume'}</button></div>
    {#if changes.length}
      <ul class="plain">
        {#each changes as c, i (i)}
          <li><label><input type="checkbox" bind:checked={ticked[i]} /> {c.text}</label></li>
        {/each}
      </ul>
      <div class="row"><button onclick={save}>Save the ticked changes</button></div>
    {/if}
    {#if resumeMsg}<p role="status">{resumeMsg}</p>{/if}
    {#if resumeError}<p role="alert">{resumeError}</p>{/if}
    {#if text.trim() !== '' && !changes.length && (resumeSaved || resumeMsg)}
      <div class="row"><button class="ghost" onclick={review} disabled={reviewing}>Get resume feedback</button></div>
    {/if}
    {#if reviewing}<p role="status">The AI is reading your resume…</p>{/if}
    {#if feedback}
      <div class="payoff" role="status">
        <strong>🎉 Here is how to make your resume stronger</strong>
        <p class="feedback">{feedback}</p>
      </div>
    {/if}
    {#if feedbackError}
      <p role="alert">{feedbackError}</p>
    {/if}
    {#if needsModel}<p class="note">Finish step 2, then choose Get resume feedback.</p>{/if}
  </li>
</ol>

<div class="row foot">
  {#if allDone}<a class="btn" href={resolve('/')} onclick={finish}>You are set. Start in chat</a>
  {:else}<a href={resolve('/')} onclick={finish}>Skip for now</a>{/if}
  <span class="note">Next: add a <a href={resolve('/job-types')}>job type</a>, and your LinkedIn <a href={resolve('/network')}>connections</a>.</span>
</div>

<style>
  .steps { display: flex; flex-direction: column; gap: 16px; max-width: 760px; padding: 0; list-style: none; }
  .step { display: flex; flex-direction: column; gap: 10px; border: 1px solid var(--line); border-radius: var(--r-md); padding: 16px; background: var(--surface); }
  .step[data-state='todo'] { opacity: 0.6; }
  .step[data-state='current'] { border-color: var(--border-control); }
  header { display: flex; align-items: center; gap: 10px; }
  header h2 { margin: 0; }
  .num { display: inline-flex; align-items: center; justify-content: center; width: 22px; height: 22px; border-radius: 50%; border: 1px solid var(--border-control); font-size: 12px; }
  .step[data-state='done'] .num { background: var(--selected); }
  textarea { font: inherit; background: var(--paper); color: var(--ink); border: 1px solid var(--border-control); padding: 12px 16px; }
  .row { display: flex; gap: 8px; flex-wrap: wrap; align-items: center; }
  .foot { max-width: 760px; margin-top: 8px; }
  ul.plain { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 8px; }
  ul.plain li { display: flex; justify-content: space-between; gap: 12px; align-items: center; flex-wrap: wrap; }
  .payoff { display: flex; flex-direction: column; gap: 8px; border: 1px solid var(--line); border-radius: var(--r-md); padding: 12px 16px; background: var(--selected); }
  .payoff p { margin: 0; }
  .feedback { white-space: pre-wrap; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; }
  .chip { height: 22px; line-height: 22px; padding: 0 10px; border-radius: var(--r-pill); background: var(--surface); border: 1px solid var(--line); font-size: 12px; }
</style>
