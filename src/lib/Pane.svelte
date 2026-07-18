<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import Terminal from './Terminal.svelte'
  import Stepper from './Stepper.svelte'
  import { store, hostIcon, type Pane } from './state.svelte'

  let {
    pane,
    active,
    onfocus,
  }: { pane: Pane; active: boolean; onfocus: () => void } = $props()

  let password = $state('')
  const Icon = $derived(pane.host ? hostIcon(pane.host) : null)

  function dotColor(phase: string): string {
    if (phase === 'connecting' || phase === 'authenticating') return 'hsl(var(--connecting))'
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
        host: pane.host.hostname,
        port: pane.host.port,
        user: pane.host.user,
        password,
      })
      password = ''
    } catch (e) {
      pane.phase = 'error'
      pane.error = String(e)
    }
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
        <!-- svelte-ignore a11y_autofocus -->
        <input type="password" placeholder="password" bind:value={password} autofocus />
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
          <h3>{#if Icon}<Icon size={18} />{/if} {pane.host.name}</h3>
          <Stepper phase={pane.phase} error={pane.error} />
          {#if pane.phase === 'disconnected'}
            <button class="btn primary" onclick={reconnect}>Reconnect</button>
          {:else if pane.phase === 'error'}
            <button class="btn primary" onclick={reset}>Try again</button>
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
