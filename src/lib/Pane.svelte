<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { ShieldAlert } from '@lucide/svelte'
  import Terminal from './Terminal.svelte'
  import Stepper from './Stepper.svelte'
  import { store, hostIcon, type Pane } from './state.svelte'

  let {
    pane,
    active,
    onfocus,
  }: { pane: Pane; active: boolean; onfocus: () => void } = $props()

  let secretVal = $state('')
  let saveSecret = $state(true)
  let hasSaved = $state(false)
  const Icon = $derived(pane.host ? hostIcon(pane.host) : null)

  // Does the keychain already hold a secret for this host? If so, skip the prompt.
  $effect(() => {
    const h = pane.host
    if (h && !pane.sessionId) {
      invoke<boolean>('secret_has', { hostId: h.id }).then((v) => (hasSaved = v))
    }
  })

  function dotColor(phase: string): string {
    if (phase === 'connecting' || phase === 'authenticating' || phase === 'hostkey')
      return 'hsl(var(--connecting))'
    if (phase === 'connected') return 'hsl(var(--primary))'
    if (phase === 'error' || phase === 'disconnected') return 'hsl(var(--destructive))'
    return 'hsl(var(--muted-foreground))'
  }

  async function connect() {
    if (!pane.host) return
    pane.error = ''
    pane.phase = 'connecting'
    try {
      pane.sessionId = await invoke<string>('ssh_connect', {
        hostId: pane.host.id,
        host: pane.host.hostname,
        port: pane.host.port,
        user: pane.host.user,
        auth: pane.host.auth,
        identityFile: pane.host.identityFile,
        secret: hasSaved ? null : secretVal,
        save: !hasSaved && saveSecret,
      })
      secretVal = ''
    } catch (e) {
      pane.phase = 'error'
      pane.error = String(e)
    }
  }

  // Answer the backend's host-key prompt. Reject makes auth fail -> Error state.
  function hostKeyDecision(accept: boolean) {
    if (!pane.sessionId) return
    invoke('ssh_host_key_decision', { id: pane.sessionId, accept })
    if (accept) pane.phase = 'connecting'
  }

  function reset() {
    if (pane.sessionId) invoke('ssh_disconnect', { id: pane.sessionId })
    pane.sessionId = null
    pane.phase = ''
    pane.error = ''
  }

  // Reuse the backend-retained credentials to restart the same session id.
  function reconnect() {
    if (!pane.sessionId) return
    pane.error = ''
    pane.phase = 'connecting'
    invoke('ssh_reconnect', { id: pane.sessionId })
  }

  // Auto-reconnect on a clean drop (not on auth errors). Fixed 2s backoff.
  // ponytail: single fixed backoff; add exponential/cap only if flaky links need it.
  $effect(() => {
    if (pane.phase === 'disconnected' && pane.host?.autoReconnect && pane.sessionId) {
      const t = setTimeout(reconnect, 2000)
      return () => clearTimeout(t)
    }
  })

  const overlay = $derived(pane.phase !== 'connected')
</script>

<div class="pane" class:active onmousedowncapture={onfocus} role="presentation">
  {#if !pane.host}
    <!-- Empty split pane: pick a host. -->
    <div class="pick">
      <p class="muted">Pick a host for this pane</p>
      <ul>
        {#each store.hosts as h (h.id)}
          {@const HIcon = hostIcon(h)}
          <li>
            <button onclick={() => (pane.host = h)}>
              <span class="hicon" style:color={h.color ?? undefined}><HIcon size={16} /></span>
              <span>{h.name}</span>
              <span class="muted mono">{h.user}@{h.hostname}</span>
            </button>
          </li>
        {:else}
          <li class="muted">No hosts saved.</li>
        {/each}
      </ul>
    </div>
  {:else if !pane.sessionId}
    <div class="connect">
      <h2>{#if Icon}<Icon size={18} />{/if} {pane.host.name}</h2>
      <p class="muted mono">{pane.host.user}@{pane.host.hostname}:{pane.host.port}</p>
      <form onsubmit={(e) => { e.preventDefault(); connect() }}>
        {#if hasSaved}
          <p class="muted small">Using saved secret from Keychain.</p>
        {:else}
          <!-- svelte-ignore a11y_autofocus -->
          <input
            type="password"
            placeholder={pane.host.auth === 'key' ? 'passphrase (if key is encrypted)' : 'password'}
            bind:value={secretVal}
            autofocus
          />
          <label class="save"><input type="checkbox" bind:checked={saveSecret} /> Save to Keychain</label>
        {/if}
        <button class="btn primary" type="submit">Connect</button>
      </form>
      {#if pane.error}<div class="err">{pane.error}</div>{/if}
    </div>
  {:else}
    <div class="wrap">
      <div class="panehead">
        <span class="dot" style:background={dotColor(pane.phase)}></span>
        {#if Icon}<Icon size={14} />{/if}
        <span>{pane.host.name}</span>
        <span class="muted mono">{pane.host.user}@{pane.host.hostname}</span>
      </div>
      <div class="term"><Terminal id={pane.sessionId} /></div>
      {#if overlay}
        <div class="cover">
          {#if pane.phase === 'hostkey'}
            <div class="hostkey" class:changed={pane.keyChanged}>
              {#if pane.keyChanged}
                <h3 class="danger"><ShieldAlert size={18} /> Host key changed for {pane.host.name}</h3>
                <p class="warn">This could indicate a man-in-the-middle attack, or the server was legitimately rebuilt. Only accept if you expected this.</p>
                <div class="fp"><span class="muted small">Previous</span><code class="mono old">{pane.oldFingerprint}</code></div>
                <div class="fp"><span class="muted small">New ({pane.keyType})</span><code class="mono">{pane.fingerprint}</code></div>
                <div class="row">
                  <button class="btn primary" onclick={() => hostKeyDecision(false)}>Reject</button>
                  <button class="btn danger-btn" onclick={() => hostKeyDecision(true)}>Accept new key</button>
                </div>
              {:else}
                <h3>{#if Icon}<Icon size={18} />{/if} Verify host key — {pane.host.name}</h3>
                <p class="muted small">First connection. Confirm this fingerprint matches the server.</p>
                <div class="fp"><span class="muted small">{pane.keyType}</span><code class="mono">{pane.fingerprint}</code></div>
                <div class="row">
                  <button class="btn" onclick={() => hostKeyDecision(false)}>Reject</button>
                  <button class="btn primary" onclick={() => hostKeyDecision(true)}>Accept</button>
                </div>
              {/if}
            </div>
          {:else}
            <h3>{#if Icon}<Icon size={18} />{/if} {pane.host.name}</h3>
            <Stepper phase={pane.phase} error={pane.error} method={pane.method} />
            {#if pane.phase === 'disconnected'}
              <button class="btn primary" onclick={reconnect}>Reconnect</button>
            {:else if pane.phase === 'error'}
              <button class="btn primary" onclick={reset}>Try again</button>
            {/if}
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .pane {
    position: relative;
    height: 100%;
    overflow: hidden;
    border: 1px solid hsl(var(--border));
    display: flex;
    flex-direction: column;
    background: hsl(var(--background));
  }
  .pane.active {
    border-color: hsl(var(--primary));
  }
  .panehead {
    height: 28px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 0 11px;
    background: hsl(var(--card));
    border-bottom: 1px solid hsl(var(--border));
    font-size: 12px;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex: none;
  }
  .wrap {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .term {
    flex: 1;
    min-height: 0;
  }
  .cover {
    position: absolute;
    inset: 0;
    background: hsl(var(--background) / 0.92);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1.2rem;
  }
  h2,
  h3 {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    font-weight: 600;
  }
  .connect {
    margin: auto;
    width: 280px;
    text-align: center;
  }
  .connect form {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    margin-top: 1rem;
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
  .pick {
    margin: auto;
    width: 90%;
    max-width: 360px;
    text-align: center;
    padding: 1rem 0;
  }
  .pick ul {
    list-style: none;
    padding: 0;
    margin: 0.6rem 0 0;
    text-align: left;
  }
  .pick button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 8px 10px;
    background: hsl(var(--card));
    border: 1px solid hsl(var(--border));
    color: inherit;
    border-radius: 7px;
    margin-bottom: 0.3rem;
    cursor: pointer;
    font-family: inherit;
    font-size: 13px;
  }
  .pick button:hover {
    background: hsl(var(--muted));
  }
  .hicon {
    display: flex;
  }
  .err {
    color: hsl(var(--destructive));
    margin-top: 0.6rem;
    font-size: 12.5px;
  }
  .small {
    font-size: 12px;
  }
  .save {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 12.5px;
    color: hsl(var(--muted-foreground));
  }
  .save input {
    width: auto;
    padding: 0;
  }
  .hostkey {
    width: 420px;
    max-width: 92%;
    text-align: center;
    padding: 1.1rem 1.3rem;
    background: hsl(var(--card));
    border: 1px solid hsl(var(--border));
    border-radius: 12px;
  }
  .hostkey.changed {
    border-color: hsl(var(--destructive));
    box-shadow: 0 0 0 1px hsl(var(--destructive) / 0.4);
  }
  .danger {
    color: hsl(var(--destructive));
  }
  .warn {
    font-size: 12.5px;
    color: hsl(var(--muted-foreground));
    margin: 0.5rem 0 0.9rem;
    line-height: 1.4;
  }
  .fp {
    display: flex;
    flex-direction: column;
    gap: 3px;
    align-items: flex-start;
    margin: 0.5rem 0;
  }
  .fp code {
    display: block;
    width: 100%;
    text-align: left;
    padding: 6px 9px;
    background: hsl(var(--muted));
    border: 1px solid hsl(var(--border));
    border-radius: 6px;
    font-size: 12px;
    word-break: break-all;
    user-select: all;
  }
  .fp code.old {
    color: hsl(var(--muted-foreground));
    text-decoration: line-through;
  }
  .row {
    display: flex;
    justify-content: center;
    gap: 0.6rem;
    margin-top: 1rem;
  }
  .btn.danger-btn {
    background: hsl(var(--destructive));
    color: #fff;
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
