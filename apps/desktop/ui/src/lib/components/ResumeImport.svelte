<script lang="ts">
  import { commands, type ProfileChange } from '$lib/bindings';
  import { accepted, savedSummary, resumeFeedbackPrompt } from '$lib/onboarding';
  import { RESUME_ACCEPT, sizeProblem } from '$lib/resume-upload';
  import { friendlyError } from '$lib/chat';

  /** Upload (or paste) a resume, choose what goes into the profile, then get feedback from the local AI. */
  let { onsaved = () => {}, showPaste = true }: { onsaved?: () => void; showPaste?: boolean } = $props();

  let input: { click(): void } | undefined = $state();
  let text = $state('');
  let fileName = $state('');
  let loadingFile = $state(false);
  let changes = $state<ProfileChange[]>([]);
  let ticked = $state<boolean[]>([]);
  let reading = $state(false);
  let message = $state('');
  let error = $state('');
  let feedback = $state('');
  let reviewing = $state(false);
  let feedbackError = $state('');
  let saved = $state(false);

  async function onFile(e: { currentTarget: { files: ArrayLike<{ name: string; size: number; arrayBuffer(): Promise<ArrayBuffer> }> | null } }) {
    const file = e.currentTarget.files?.[0];
    if (!file) return;
    error = message = feedback = feedbackError = '';
    const big = sizeProblem(file.size);
    if (big) {
      error = big;
      return;
    }
    loadingFile = true;
    try {
      const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
      const r = await commands.readResumeFile(file.name, bytes);
      if (r.status === 'ok') {
        text = r.data;
        fileName = file.name;
        await read();
      } else error = r.error;
    } catch (err) {
      error = `Could not open that file: ${String(err)}`;
    } finally {
      loadingFile = false;
    }
  }

  async function read() {
    error = message = '';
    reading = true;
    try {
      const r = await commands.previewResumeImport(text);
      if (r.status === 'ok') {
        changes = r.data;
        ticked = r.data.map(() => true);
        if (r.data.length === 0) {
          message = 'Your profile already has everything in this resume.';
          await markSaved();
          await review();
        }
      } else {
        const f = friendlyError(r.error);
        error = f.needsModel ? 'Reading your resume needs the local AI. Download it first, then try again.' : r.error;
      }
    } catch (e) {
      error = String(e);
    } finally {
      reading = false;
    }
  }

  /** The profile now holds the resume: keep a base resume in Documents in step with it. */
  async function markSaved() {
    saved = true;
    try {
      const r = await commands.saveBaseResume();
      if (r.status !== 'ok') error = r.error;
    } catch {
      // Outside the desktop app there is no store to write to.
    }
    onsaved();
  }

  async function save() {
    error = '';
    const r = await commands.applyProfileChanges(accepted(changes, ticked));
    if (r.status === 'ok') {
      message = savedSummary(r.data);
      changes = [];
      ticked = [];
      await markSaved();
      await review();
    } else error = r.error;
  }

  /** The payoff: ask the local AI how to make the resume better. */
  async function review() {
    reviewing = true;
    feedbackError = '';
    try {
      const r = await commands.askModel(resumeFeedbackPrompt(text));
      if (r.status === 'ok') feedback = r.data.trim();
      else feedbackError = friendlyError(r.error).text;
    } catch (e) {
      feedbackError = String(e);
    } finally {
      reviewing = false;
    }
  }
</script>

<div class="import">
  <div class="row">
    <button onclick={() => input?.click()} disabled={loadingFile || reading}>{loadingFile ? 'Opening' : 'Upload resume'}</button>
    <span class="note">PDF, Word, text or Markdown. It is read on this computer.</span>
  </div>
  <input class="hidden" type="file" accept={RESUME_ACCEPT} aria-label="Choose a resume file" bind:this={input} onchange={onFile} />
  {#if fileName}<p class="note">Reading {fileName}</p>{/if}
  {#if showPaste}
    <details>
      <summary>Or paste the text</summary>
      <textarea aria-label="Resume text" rows="8" bind:value={text}></textarea>
      <div class="row"><button class="ghost" onclick={read} disabled={reading || text.trim() === ''}>{reading ? 'Reading' : 'Read my resume'}</button></div>
    </details>
  {/if}
  {#if reading}<p role="status">Reading your resume…</p>{/if}
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
  {#if saved && !feedback && !reviewing}
    <div class="row"><button class="ghost" onclick={review}>Get resume feedback</button></div>
  {/if}
  {#if reviewing}<p role="status">The AI is reading your resume…</p>{/if}
  {#if feedback}
    <div class="payoff" role="status">
      <strong>🎉 Here is how to make your resume stronger</strong>
      <p class="feedback">{feedback}</p>
    </div>
  {/if}
  {#if feedbackError}<p role="alert">{feedbackError}</p>{/if}
</div>

<style>
  .import { display: flex; flex-direction: column; gap: 10px; }
  .hidden { display: none; }
  .row { display: flex; gap: 8px; flex-wrap: wrap; align-items: center; }
  textarea { font: inherit; width: 100%; box-sizing: border-box; background: var(--paper); color: var(--ink); border: 1px solid var(--border-control); padding: 12px 16px; margin-top: 8px; }
  ul.plain { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 8px; }
  .payoff { display: flex; flex-direction: column; gap: 8px; border: 1px solid var(--line); border-radius: var(--r-md); padding: 12px 16px; background: var(--selected); }
  .payoff p { margin: 0; }
  .feedback { white-space: pre-wrap; }
</style>
