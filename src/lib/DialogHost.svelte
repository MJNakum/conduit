<script lang="ts">
  import { tick } from 'svelte'
  import { dialogState, settleDialog } from './dialog.svelte'
  import { trapFocus } from './actions/trapFocus'

  let value = $state('')
  let repeat = $state('')
  let input = $state<HTMLInputElement>()

  // Seed the prompt field and focus it whenever a new request appears.
  $effect(() => {
    const req = dialogState.current
    if (req?.kind === 'prompt') {
      value = req.value
      repeat = ''
      tick().then(() => {
        input?.focus()
        input?.select()
      })
    }
  })

  // A confirm-style prompt (setting a passphrase) can't be submitted until both
  // fields agree, and an empty passphrase is never worth accepting.
  const blocked = $derived.by(() => {
    const req = dialogState.current
    if (req?.kind !== 'prompt' || !req.confirm) return false
    return value.length === 0 || value !== repeat
  })

  function ok() {
    const req = dialogState.current
    if (req?.kind !== 'prompt') return settleDialog(true)
    if (blocked) return
    // Passphrases are taken verbatim: a trailing space is part of the secret.
    settleDialog(req.password ? value : value.trim())
  }
  function cancel() {
    settleDialog(dialogState.current?.kind === 'prompt' ? null : false)
  }
</script>

{#if dialogState.current}
  {@const req = dialogState.current}
  <div class="backdrop" onclick={cancel} role="presentation">
    <div
      class="dialog"
      onclick={(e) => e.stopPropagation()}
      role="dialog"
      tabindex="-1"
      aria-modal="true"
      aria-label={req.title}
      use:trapFocus={{ onclose: cancel }}
    >
      <h2>{req.title}</h2>
      {#if req.message}<p class="msg">{req.message}</p>{/if}
      {#if req.kind === 'prompt'}
        <input
          bind:this={input}
          bind:value
          type={req.password ? 'password' : 'text'}
          placeholder={req.placeholder ?? ''}
          onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); ok() } }}
        />
        {#if req.confirm}
          <input
            bind:value={repeat}
            type={req.password ? 'password' : 'text'}
            placeholder="Repeat"
            onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); ok() } }}
          />
          {#if repeat.length > 0 && value !== repeat}
            <p class="mismatch">These do not match.</p>
          {/if}
        {/if}
      {/if}
      <div class="row">
        <button class="btn ghost" onclick={cancel}>Cancel</button>
        <button class="btn" class:danger={req.danger} onclick={ok} disabled={blocked}>
          {req.okLabel ?? 'OK'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 20vh;
    z-index: 80;
  }
  .dialog {
    width: 420px;
    max-width: 92vw;
    background: hsl(var(--popover));
    border: 1px solid hsl(var(--border));
    border-radius: 12px;
    padding: 18px 20px;
    box-shadow: 0 24px 64px rgba(0, 0, 0, 0.55);
  }
  h2 {
    margin: 0 0 6px;
    font-size: 14.5px;
    font-weight: 600;
  }
  .msg {
    margin: 0 0 12px;
    font-size: 13px;
    color: hsl(var(--muted-foreground));
    white-space: pre-wrap;
    word-break: break-word;
  }
  input {
    width: 100%;
    background: hsl(var(--muted));
    border: 1px solid hsl(var(--border));
    border-radius: 7px;
    padding: 8px 10px;
    color: inherit;
    outline: none;
    font-size: 13px;
    font-family: inherit;
    margin-bottom: 14px;
  }
  input:focus {
    border-color: hsl(var(--ring) / 0.6);
  }
  .mismatch {
    margin: -8px 0 12px;
    font-size: 12px;
    color: hsl(var(--destructive));
  }
  .row {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .btn {
    padding: 8px 14px;
    border: none;
    border-radius: 7px;
    background: hsl(var(--primary));
    color: hsl(var(--primary-foreground));
    font: inherit;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }
  .btn.ghost {
    background: transparent;
    color: hsl(var(--foreground));
    font-weight: 400;
  }
  .btn.ghost:hover {
    background: hsl(var(--muted));
  }
  .btn.danger {
    background: hsl(var(--destructive));
    color: #fff;
  }
  .btn:hover {
    filter: brightness(1.08);
  }
  .btn:disabled {
    opacity: 0.5;
    cursor: default;
    filter: none;
  }
</style>
