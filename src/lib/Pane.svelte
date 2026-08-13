<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { ScrollText, ShieldAlert, ShieldCheck } from '@lucide/svelte'
  import Terminal from './Terminal.svelte'
  import Stepper from './Stepper.svelte'
  import Challenge from './Challenge.svelte'
  import RawLog from './RawLog.svelte'
  import { store, ui, broadcast, hostIcon, resolveJumps, type Pane } from './state.svelte'
  import { secretsState, ensureUsable, storeName } from './secrets.svelte'
  import { resolveScheme, xtermTheme, settings } from './theme.svelte'
  import { toast } from './toast.svelte'

  let {
    pane,
    active,
    onfocus,
  }: { pane: Pane; active: boolean; onfocus: () => void } = $props()

  let secretVal = $state('')
  let saveSecret = $state(true)
  let hasSaved = $state(false)
  let showRaw = $state(false)
  const Icon = $derived(pane.host ? hostIcon(pane.host) : null)
  // Track the most-recently-active connected session so snippet "Run" has a target.
  $effect(() => {
    if (active && pane.phase === 'connected' && pane.sessionId) ui.lastSession = pane.sessionId
  })
  // Managed keys carry no secret (private material is unencrypted in the keychain),
  // so key-auth-with-a-managed-key needs no prompt.
  const managedKey = $derived(pane.host?.auth === 'key' && !!pane.host?.keyId)
  const isTelnet = $derived(pane.host?.protocol === 'telnet')
  // Telnet has no client auth; SSH managed-key/saved-secret also skip the prompt.
  const promptSecret = $derived(!isTelnet && !hasSaved && !managedKey)

  // Only a hint for the save checkbox. Deliberately does not fetch the status —
  // that would force a backend probe on the connect path, and connection latency
  // is the metric that matters. It reads as false until the user has visited the
  // Keys or Settings page; `ensureUsable()` still does the real work at save time.
  const storeLocked = $derived(
    secretsState.status?.kind === 'file' &&
      (secretsState.status.locked || secretsState.status.uninitialized),
  )

  // Does the keychain already hold a password for this host? Only relevant for
  // password-auth SSH — probing key/telnet hosts would trigger a needless
  // Keychain prompt for a slot they never use.
  $effect(() => {
    const h = pane.host
    if (h && !pane.sessionId && h.auth === 'password' && h.protocol !== 'telnet') {
      invoke<boolean>('secret_has', { hostId: h.id }).then((v) => (hasSaved = v))
    } else {
      hasSaved = false
    }
  })

  function dotColor(phase: string): string {
    if (phase === 'connecting' || phase === 'authenticating' || phase === 'hostkey')
      return 'hsl(var(--connecting))'
    if (phase === 'connected') return 'hsl(var(--primary))'
    if (phase === 'error' || phase === 'disconnected') return 'hsl(var(--destructive))'
    return 'hsl(var(--muted-foreground))'
  }

  // Begin a fresh attempt: the log that just failed moves aside rather than
  // being dropped, so it's still readable after a retry behaves differently.
  function startAttempt() {
    pane.error = ''
    pane.phase = 'connecting'
    if (pane.connLog.length) pane.prevLog = pane.connLog
    pane.connLog = []
    pane.activeStep = 'connecting'
    pane.kb = null
    pane.sawMfa = false
    showRaw = false
  }

  async function connect() {
    if (!pane.host) return
    startAttempt()
    try {
      if (isTelnet) {
        pane.sessionId = await invoke<string>('telnet_connect', {
          host: pane.host.hostname,
          port: pane.host.port,
        })
        return
      }
      pane.sessionId = await invoke<string>('ssh_connect', {
        hostId: pane.host.id,
        host: pane.host.hostname,
        port: pane.host.port,
        user: pane.host.user,
        auth: pane.host.auth,
        keyId: pane.host.keyId,
        identityFile: pane.host.identityFile,
        // Only send a secret when we actually prompted for one. Otherwise
        // (managed key, telnet, or an already-saved secret) leave it null so we
        // never write a junk empty entry or re-read the keychain needlessly.
        secret: promptSecret ? secretVal : null,
        // Saving is done here instead, so a failure is visible — the backend
        // had no way to report one without aborting a good connection.
        save: false,
        jumps: resolveJumps(pane.host),
        logName: pane.host.logging ? pane.host.name : null,
      })
      const toSave = promptSecret && saveSecret ? secretVal : ''
      secretVal = ''
      // After the connect call, so a storage passphrase prompt never delays the
      // connection itself. ponytail: this still saves before authentication has
      // been confirmed, so a mistyped password gets stored — same as before this
      // change. Saving on the `connected` state event is the real fix.
      if (toSave) void saveSecretFor(pane.host.id, toSave)
    } catch (e) {
      pane.phase = 'error'
      pane.error = String(e)
    }
  }

  // Report what actually happened. The old code toasted success unconditionally
  // while the backend discarded the result, so on a machine with no working
  // secret storage it claimed to have saved a password it had dropped.
  async function saveSecretFor(hostId: string, secret: string) {
    try {
      if (!(await ensureUsable())) {
        toast('Password not saved — secret storage was not unlocked')
        return
      }
      await invoke('secret_set', { hostId, secret })
      hasSaved = true
      toast(`Password saved to ${storeName()}`)
    } catch (e) {
      toast(`Password not saved: ${String(e)}`)
    }
  }

  // Drop the saved secret so the next connect prompts again.
  async function forgetSecret() {
    if (!pane.host) return
    await invoke('secret_delete', { hostId: pane.host.id })
    hasSaved = false
    toast('Saved password removed')
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
    pane.prevLog = pane.connLog
    pane.connLog = []
    pane.activeStep = 'connecting'
    pane.kb = null
    pane.sawMfa = false
    showRaw = false
  }

  // Reuse the backend-retained credentials to restart the same session id.
  function reconnect() {
    if (!pane.sessionId) return
    startAttempt()
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

  const overlay = $derived(pane.phase !== 'connected' || showRaw)
  const failed = $derived(pane.phase === 'error' || pane.phase === 'disconnected')
  // Amber outline when this connected pane is a live broadcast target (design §9).
  const broadcasting = $derived(
    broadcast.on &&
      pane.phase === 'connected' &&
      !!pane.sessionId &&
      !broadcast.exclude.includes(pane.sessionId),
  )
</script>

<div class="pane" class:active class:broadcasting onmousedowncapture={onfocus} role="presentation">
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
        {#if !promptSecret}
          <p class="muted small">
            {isTelnet ? 'Telnet — no authentication.' : managedKey ? 'Authenticating with a managed key.' : `Using saved secret from ${storeName()}.`}
            {#if hasSaved}
              <button type="button" class="linkbtn" onclick={forgetSecret}>Forget</button>
            {/if}
          </p>
        {:else}
          <!-- svelte-ignore a11y_autofocus -->
          <input
            type="password"
            placeholder={pane.host.auth === 'key' ? 'passphrase (if key is encrypted)' : 'password'}
            bind:value={secretVal}
            autofocus
          />
          <label class="save"><input type="checkbox" bind:checked={saveSecret} /> Save to {storeName()}</label>
          {#if storeLocked}
            <p class="muted small">
              Secret storage is locked, so a saved password would be lost. You will be asked to
              unlock it.
            </p>
          {/if}
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
        <span class="spacer"></span>
        <!-- The trace stays reachable after a successful connect: the case you
             most need it for is a session that authenticated and then dropped.
             Hidden while a prompt is up, since that takes over the overlay. -->
        {#if !pane.kb && pane.phase !== 'hostkey'}
          <button
            class="headbtn"
            onclick={() => (showRaw = !showRaw)}
            title="Connection log"
            aria-label="Connection log"
          >
            <ScrollText size={13} />
          </button>
        {/if}
      </div>
      <div class="term">
        <Terminal
          id={pane.sessionId}
          theme={xtermTheme(resolveScheme(pane.host.scheme))}
          fontFamily={pane.host.font || settings.defaultFont}
          fontSize={pane.host.fontSize || settings.defaultFontSize}
        />
      </div>
      {#if overlay}
        <div class="cover">
          {#if pane.phase === 'hostkey'}
            <div class="hostkey" class:changed={pane.keyChanged}>
              {#if pane.keyChanged}
                <h3 class="danger"><ShieldAlert size={18} /> Host key changed for {pane.keyHost || pane.host.name}</h3>
                <p class="warn">This could indicate a man-in-the-middle attack, or the server was legitimately rebuilt. Only accept if you expected this.</p>
                <div class="fp"><span class="muted small">Previous</span><code class="mono old">{pane.oldFingerprint}</code></div>
                <div class="fp"><span class="muted small">New ({pane.keyType})</span><code class="mono">{pane.fingerprint}</code></div>
                <div class="row">
                  <button class="btn primary" onclick={() => hostKeyDecision(false)}>Reject</button>
                  <button class="btn danger-btn" onclick={() => hostKeyDecision(true)}>Accept new key</button>
                </div>
              {:else}
                <h3>{#if Icon}<Icon size={18} />{/if} Verify host key — {pane.keyHost || pane.host.name}</h3>
                <p class="muted small">First connection{pane.keyHost && pane.keyHost !== pane.host.hostname ? ' (jump host)' : ''}. Confirm this fingerprint matches the server.</p>
                <div class="fp"><span class="muted small">{pane.keyType}</span><code class="mono">{pane.fingerprint}</code></div>
                <div class="row">
                  <button class="btn" onclick={() => hostKeyDecision(false)}>Reject</button>
                  <button class="btn primary" onclick={() => hostKeyDecision(true)}>Accept</button>
                </div>
              {/if}
            </div>
          {:else if pane.kb}
            <!-- Keyboard-interactive challenge, in place as a step rather than a
                 modal (design-spec §5): this is where a verification code is
                 entered. Keyed so a re-ask arrives with empty fields. -->
            <div class="hostkey">
              <h3><ShieldCheck size={18} /> {pane.kb.name.trim() || 'Verification required'}</h3>
              <p class="muted small mono">{pane.kb.label}</p>
              {#key pane.kb.prompt_id}
                <Challenge prompt={pane.kb} autofocus />
              {/key}
            </div>
          {:else}
            <h3>{#if Icon}<Icon size={18} />{/if} {pane.host.name}</h3>
            {#if showRaw}
              <RawLog
                log={pane.connLog}
                previous={pane.prevLog}
                host={`${pane.host.user}@${pane.host.hostname}:${pane.host.port}`}
                name={pane.host.name}
                onclose={() => (showRaw = false)}
              />
            {:else}
              <Stepper
                phase={pane.phase}
                error={pane.error}
                method={pane.method}
                protocol={pane.host.protocol ?? 'ssh'}
                mfa={pane.sawMfa}
                log={pane.connLog}
                activeStep={pane.activeStep}
              />
            {/if}
            {#if failed}
              <div class="actions">
                {#if pane.phase === 'disconnected'}
                  <button class="btn primary" onclick={reconnect}>Reconnect</button>
                {:else}
                  <button class="btn primary" onclick={reset}>Try again</button>
                {/if}
                {#if !showRaw}
                  <button class="btn" onclick={() => (showRaw = true)}>Raw log</button>
                {/if}
              </div>
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
  .pane.broadcasting {
    border-color: hsl(var(--amber));
    box-shadow: inset 0 0 0 1px hsl(var(--amber));
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
  .spacer {
    flex: 1;
  }
  .headbtn {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    flex: none;
    border: none;
    border-radius: 5px;
    background: none;
    color: hsl(var(--muted-foreground));
    cursor: pointer;
  }
  .headbtn:hover {
    background: hsl(var(--muted));
    color: hsl(var(--foreground));
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
  .linkbtn {
    background: none;
    border: 0;
    padding: 0;
    margin-left: 6px;
    font: inherit;
    color: hsl(var(--primary));
    cursor: pointer;
    text-decoration: underline;
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
  .actions {
    display: flex;
    justify-content: center;
    gap: 0.6rem;
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
