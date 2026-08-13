<script lang="ts">
  import { onMount } from 'svelte'
  import { ShieldCheck, ShieldAlert, Lock, LockOpen, Terminal, Copy, Info } from '@lucide/svelte'
  import { secretsState, loadStatus, ensureUsable, pinBackend } from './secrets.svelte'
  import { toast } from './toast.svelte'
  import { confirmDialog } from './dialog.svelte'

  // Opening this page is a deliberate visit to credential UI, so this is the
  // right place to pay for the backend probe (see secrets.svelte.ts).
  onMount(loadStatus)

  const s = $derived(secretsState.status)
  const keyringOk = $derived(s?.kind === 'keyring')

  function copy(text: string) {
    navigator.clipboard?.writeText(text)
    toast('Copied to clipboard')
  }

  async function setUpNow() {
    if (await ensureUsable()) {
      await loadStatus()
      toast('Secret storage ready')
    }
  }

  async function switchTo(kind: 'file' | 'keyring') {
    const to = kind === 'file' ? 'the encrypted file' : 'the system keyring'
    const ok = await confirmDialog({
      title: 'Change secret storage',
      message:
        `Conduit will use ${to} the next time it starts.\n\n` +
        'Secrets already saved in the other store stay where they are — nothing is copied across, ' +
        'and you can switch back at any time.',
      okLabel: 'Use it next launch',
    })
    if (!ok) return
    await pinBackend(kind)
    toast(`Conduit will use ${to} after a restart`)
  }

  async function clearPin() {
    await pinBackend(null)
    toast('Conduit will choose automatically again')
  }

  // Shell snippets. Each is copyable because retyping a command from a UI is
  // exactly where typos come from.
  const install = [
    { label: 'GNOME, Ubuntu, Pop!_OS', cmd: 'sudo apt install gnome-keyring' },
    { label: 'KDE Plasma', cmd: 'sudo apt install kwalletmanager' },
    { label: 'Fedora', cmd: 'sudo dnf install gnome-keyring' },
    { label: 'Arch', cmd: 'sudo pacman -S gnome-keyring' },
  ]
  const ping =
    'gdbus call --session --dest org.freedesktop.secrets \\\n  --object-path /org/freedesktop/secrets \\\n  --method org.freedesktop.DBus.Peer.Ping'
  const start = 'gnome-keyring-daemon --start --components=secrets'
</script>

<div class="pane">
  {#if !s}
    <p class="muted">Checking secret storage…</p>
  {:else}
    <section class="card" class:warn={!keyringOk}>
      <div class="head">
        {#if keyringOk}
          <ShieldCheck size={16} />
        {:else}
          <ShieldAlert size={16} />
        {/if}
        <span class="name">{s.label}</span>
        {#if s.kind === 'file'}
          <span class="state">
            {#if s.uninitialized}
              <Lock size={12} /> Not set up
            {:else if s.locked}
              <Lock size={12} /> Locked
            {:else}
              <LockOpen size={12} /> Unlocked
            {/if}
          </span>
        {/if}
      </div>
      <p class="detail">{s.detail}</p>

      {#if s.kind === 'file' && (s.locked || s.uninitialized)}
        <div class="acts">
          <button class="btn primary" onclick={setUpNow} disabled={secretsState.busy}>
            {s.uninitialized ? 'Set a passphrase' : 'Unlock'}
          </button>
        </div>
      {/if}

      {#if s.pinned}
        <p class="pinned">
          <Info size={13} />
          You chose this store manually.
          <button class="link" onclick={clearPin}>Choose automatically instead</button>
        </p>
      {/if}
    </section>

    {#if s.linux}
      {#if !keyringOk}
        <section class="block">
          <h3>Set up a system keyring</h3>
          <p>
            Conduit prefers your desktop's Secret Service — GNOME Keyring, KWallet, or KeePassXC —
            because it is already unlocked when you log in. Install one, then log out and back in so
            it starts with your session.
          </p>
          <ul class="cmds">
            {#each install as row}
              <li>
                <span class="cmd-label">{row.label}</span>
                <code class="mono">{row.cmd}</code>
                <button class="icon" title="Copy" onclick={() => copy(row.cmd)}><Copy size={13} /></button>
              </li>
            {/each}
          </ul>
          <p class="note">
            Using KeePassXC? Turn on Tools &rarr; Settings &rarr; Secret Service Integration and expose a
            group.
          </p>

          <h3>Installed, but Conduit still cannot see it</h3>
          <ul class="bullets">
            <li>
              <strong>It has to be unlocked.</strong> On GNOME the login keyring unlocks with your
              password — but not if you log in automatically, or if the keyring's password differs from
              your login password. Open Passwords and Keys (<code class="mono">seahorse</code>) and unlock
              the <em>Login</em> keyring.
            </li>
            <li>
              <strong>Check whether it is on the bus.</strong> A reply means it is running.
              <div class="snippet">
                <Terminal size={13} />
                <code class="mono block">{ping}</code>
                <button class="icon" title="Copy" onclick={() => copy(ping)}><Copy size={13} /></button>
              </div>
            </li>
            <li>
              <strong>Start it for this session.</strong>
              <div class="snippet">
                <Terminal size={13} />
                <code class="mono block">{start}</code>
                <button class="icon" title="Copy" onclick={() => copy(start)}><Copy size={13} /></button>
              </div>
            </li>
            <li>
              <strong>Headless, over SSH, or on a bare tty?</strong> There is no D-Bus session bus, so
              there is no Secret Service to find. The encrypted file is the right choice there — no
              amount of installing will change it.
            </li>
          </ul>
        </section>
      {/if}

      <section class="block">
        <h3>How the encrypted file works</h3>
        <p>
          Secrets are sealed with AES-256-GCM under a key derived from your passphrase with Argon2id,
          in a file only your user can read. Conduit asks for the passphrase once per run, the first
          time it needs a secret. There is no recovery: forget it and the saved passwords and private
          keys are gone.
        </p>
        {#if keyringOk && s.store_file_exists}
          <p class="note">
            You have an encrypted file from an earlier session. Its secrets are not visible while the
            system keyring is in use — nothing is copied between the two.
          </p>
        {/if}
        <div class="acts">
          {#if keyringOk}
            <button class="btn" onclick={() => switchTo('file')}>Use the encrypted file instead</button>
          {:else}
            <button class="btn" onclick={() => switchTo('keyring')}>Use the system keyring instead</button>
          {/if}
        </div>
      </section>
    {/if}
  {/if}
</div>

<style>
  .pane {
    display: flex;
    flex-direction: column;
    gap: 20px;
    max-width: 640px;
  }
  .card {
    border: 1px solid hsl(var(--border));
    border-radius: 9px;
    padding: 14px 16px;
  }
  .card.warn {
    border-color: hsl(var(--destructive) / 0.45);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 9px;
  }
  .head .name {
    font-size: 14px;
    font-weight: 600;
  }
  .state {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    margin-left: auto;
    padding: 3px 8px;
    border-radius: 999px;
    background: hsl(var(--muted));
    color: hsl(var(--muted-foreground));
    font-size: 11.5px;
  }
  .detail {
    margin: 8px 0 0;
    font-size: 13px;
    color: hsl(var(--muted-foreground));
  }
  .pinned {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 10px 0 0;
    font-size: 12px;
    color: hsl(var(--muted-foreground));
  }
  .block h3 {
    margin: 0 0 8px;
    font-size: 13.5px;
    font-weight: 600;
  }
  .block h3:not(:first-child) {
    margin-top: 22px;
  }
  .block p {
    margin: 0 0 12px;
    font-size: 13px;
    line-height: 1.55;
    color: hsl(var(--muted-foreground));
  }
  .note {
    font-size: 12.5px;
  }
  .cmds,
  .bullets {
    margin: 0 0 12px;
    padding: 0;
    list-style: none;
  }
  .bullets {
    padding-left: 16px;
    list-style: disc;
  }
  .bullets li {
    margin-bottom: 12px;
    font-size: 13px;
    line-height: 1.55;
    color: hsl(var(--muted-foreground));
  }
  .bullets strong {
    color: hsl(var(--foreground));
    font-weight: 600;
  }
  .cmds li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 0;
  }
  .cmd-label {
    flex: none;
    width: 170px;
    font-size: 12.5px;
    color: hsl(var(--muted-foreground));
  }
  .snippet {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    margin: 8px 0 0;
  }
  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 12px;
  }
  code.mono {
    flex: 1;
    min-width: 0;
    padding: 5px 8px;
    border-radius: 6px;
    background: hsl(var(--muted));
    color: hsl(var(--foreground));
    overflow-x: auto;
  }
  code.mono.block {
    white-space: pre;
  }
  .acts {
    display: flex;
    gap: 8px;
    margin-top: 12px;
  }
  .btn {
    padding: 7px 13px;
    border: 1px solid hsl(var(--border));
    border-radius: 7px;
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
  .icon {
    flex: none;
    display: inline-flex;
    align-items: center;
    padding: 5px;
    border: none;
    border-radius: 6px;
    background: none;
    color: hsl(var(--muted-foreground));
    cursor: pointer;
  }
  .icon:hover {
    background: hsl(var(--muted));
    color: hsl(var(--foreground));
  }
  .link {
    border: none;
    background: none;
    padding: 0;
    color: hsl(var(--primary));
    font: inherit;
    font-size: 12px;
    text-decoration: underline;
    cursor: pointer;
  }
  .muted {
    color: hsl(var(--muted-foreground));
    font-size: 13px;
  }
</style>
