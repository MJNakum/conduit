<script lang="ts">
  // Launch-time unlock for the encrypted file store. Without this the first
  // thing a user meets is a connection failing at the auth step with "could not
  // read this key from secret storage", and the only cure was hunting through
  // the app for the unlock button. Ask once, up front, while it is obvious why.
  //
  // Only shown when a store actually exists and is locked. A store that has
  // never been set up holds nothing worth prompting for, and the keyring
  // backends are unlocked by the desktop at login.
  import { Lock } from '@lucide/svelte'
  import { secretsState, unlockWith, dismissUnlockPrompt } from './secrets.svelte'
  import { trapFocus } from './actions/trapFocus'
  import { toast } from './toast.svelte'

  let passphrase = $state('')
  let error = $state('')
  let input = $state<HTMLInputElement>()

  $effect(() => {
    input?.focus()
  })

  async function submit() {
    if (!passphrase || secretsState.busy) return
    error = ''
    const failure = await unlockWith(passphrase)
    if (failure) {
      error = failure
      passphrase = ''
      input?.focus()
      return
    }
    toast('Secret storage unlocked')
  }
</script>

<div class="lock">
  <div
    class="card"
    role="dialog"
    aria-modal="true"
    aria-label="Unlock secret storage"
    tabindex="-1"
    use:trapFocus={{}}
    onkeydown={(e) => { if (e.key === 'Escape') { e.preventDefault(); dismissUnlockPrompt() } }}
  >
    <span class="ico"><Lock size={28} /></span>
    <h2>Unlock secret storage</h2>
    <p class="muted">
      Your saved passwords and private keys are in an encrypted file. Conduit needs its passphrase
      before it can connect with them.
    </p>

    <input
      bind:this={input}
      bind:value={passphrase}
      type="password"
      placeholder="Passphrase"
      aria-label="Passphrase"
      onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); submit() } }}
    />
    {#if error}<p class="err">{error}</p>{/if}

    <div class="row">
      <button class="btn ghost" onclick={dismissUnlockPrompt}>Not now</button>
      <button class="btn primary" onclick={submit} disabled={!passphrase || secretsState.busy}>
        {secretsState.busy ? 'Unlocking…' : 'Unlock'}
      </button>
    </div>
    <p class="hint muted">
      You can skip this — Conduit will ask again the first time it needs a secret.
    </p>
  </div>
</div>

<style>
  /* Same treatment as the vault LockScreen so the two read as one idea. */
  .lock {
    position: fixed;
    inset: 0;
    z-index: 92;
    display: grid;
    place-items: center;
    background: hsl(var(--background) / 0.72);
    backdrop-filter: blur(18px);
  }
  .card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    width: min(380px, calc(100vw - 48px));
    padding: 30px 34px;
    border: 1px solid hsl(var(--border));
    border-radius: 16px;
    background: hsl(var(--card));
    box-shadow: 0 30px 80px rgba(0, 0, 0, 0.5);
    text-align: center;
  }
  .ico {
    display: grid;
    place-items: center;
    width: 56px;
    height: 56px;
    border-radius: 50%;
    background: hsl(var(--primary) / 0.12);
    color: hsl(var(--primary));
  }
  h2 {
    margin: 4px 0 0;
    font-size: 17px;
  }
  p {
    margin: 0;
    font-size: 13px;
    line-height: 1.5;
  }
  .muted {
    color: hsl(var(--muted-foreground));
  }
  input {
    width: 100%;
    margin-top: 4px;
    padding: 9px 11px;
    border: 1px solid hsl(var(--border));
    border-radius: 8px;
    background: hsl(var(--background));
    color: hsl(var(--foreground));
    font: inherit;
    font-size: 13px;
  }
  input:focus {
    outline: none;
    border-color: hsl(var(--ring) / 0.6);
  }
  .err {
    align-self: flex-start;
    color: hsl(var(--destructive));
    font-size: 12px;
    text-align: left;
  }
  .row {
    display: flex;
    gap: 8px;
    width: 100%;
    margin-top: 4px;
  }
  .btn {
    flex: 1;
    padding: 9px 14px;
    border: 1px solid hsl(var(--border));
    border-radius: 8px;
    background: hsl(var(--muted));
    color: hsl(var(--foreground));
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .btn.primary {
    background: hsl(var(--primary));
    border-color: transparent;
    color: hsl(var(--primary-foreground));
    font-weight: 600;
  }
  .btn:hover {
    filter: brightness(1.08);
  }
  .btn:disabled {
    opacity: 0.55;
    cursor: default;
    filter: none;
  }
  .hint {
    font-size: 11.5px;
  }
</style>
