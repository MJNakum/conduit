<script lang="ts">
  import { tick } from 'svelte'
  import { answerPrompt, type PromptPayload } from './state.svelte'

  // The server's keyboard-interactive challenge, rendered as a form. Shared by
  // the in-pane card (a connection a pane owns) and the global dialog (a port
  // forward or SFTP connection, which has no pane). Answers go straight to the
  // backend and are never stored — they are one-time by definition, so there is
  // deliberately no "save to Keychain" here.
  let { prompt, autofocus = false }: { prompt: PromptPayload; autofocus?: boolean } = $props()

  let values = $state<string[]>([])
  let inputs = $state<HTMLInputElement[]>([])

  // Reset when a new challenge arrives (a wrong code re-asks with fresh fields).
  $effect(() => {
    values = prompt.fields.map(() => '')
    if (autofocus) tick().then(() => inputs[0]?.focus())
  })

  // A one-time code is numeric and worth an OTP keyboard on the field; a
  // password is not. Matched on the server's own wording.
  const isCode = (text: string) => /code|token|otp|one[- ]time/i.test(text)

  function submit() {
    answerPrompt(prompt.prompt_id, values)
  }
  function cancel() {
    answerPrompt(prompt.prompt_id, null)
  }
</script>

<form onsubmit={(e) => { e.preventDefault(); submit() }}>
  {#if prompt.instruction.trim()}
    <!-- Selectable: PAM instructions routinely carry an enrolment URL or a
         support contact, which is useless if it can't be copied. -->
    <p class="instruction">{prompt.instruction}</p>
  {/if}
  {#each prompt.fields as field, i (i)}
    <label>
      <span class="ftext">{field.prompt.trim()}</span>
      <input
        bind:this={inputs[i]}
        bind:value={values[i]}
        type={field.echo ? 'text' : 'password'}
        inputmode={isCode(field.prompt) ? 'numeric' : undefined}
        autocomplete={isCode(field.prompt) ? 'one-time-code' : 'off'}
        spellcheck="false"
      />
    </label>
  {/each}
  <div class="row">
    <button class="btn" type="button" onclick={cancel}>Cancel</button>
    <button class="btn primary" type="submit">Continue</button>
  </div>
</form>

<style>
  form {
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
    text-align: left;
  }
  .instruction {
    margin: 0;
    font-size: 12.5px;
    line-height: 1.45;
    color: hsl(var(--muted-foreground));
    white-space: pre-wrap;
    word-break: break-word;
    user-select: text;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .ftext {
    font-size: 12px;
    color: hsl(var(--muted-foreground));
    user-select: text;
  }
  input {
    padding: 8px 10px;
    background: hsl(var(--muted));
    border: 1px solid hsl(var(--border));
    color: inherit;
    border-radius: 7px;
    outline: none;
    font-family: inherit;
    font-size: 13px;
  }
  input:focus {
    border-color: hsl(var(--ring) / 0.6);
  }
  .row {
    display: flex;
    justify-content: flex-end;
    gap: 0.6rem;
    margin-top: 0.3rem;
  }
  .btn {
    padding: 8px 14px;
    border: none;
    border-radius: 7px;
    background: hsl(var(--muted));
    color: inherit;
    font-family: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .btn.primary {
    background: hsl(var(--primary));
    color: hsl(var(--primary-foreground));
    font-weight: 600;
  }
  .btn.primary:hover {
    filter: brightness(1.08);
  }
</style>
