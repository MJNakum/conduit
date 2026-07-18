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
      <p class="dim">Pick a host for this pane</p>
      <ul>
        {#each store.hosts as h (h.id)}
          {@const HIcon = hostIcon(h)}
          <li>
            <button onclick={() => (pane.host = h)}>
              <span class="hicon" style:color={h.color ?? undefined}><HIcon size={16} /></span>
              <span>{h.name}</span>
              <span class="dim">{h.user}@{h.hostname}</span>
            </button>
          </li>
        {:else}
          <li class="dim">No hosts saved.</li>
        {/each}
      </ul>
    </div>
  {:else if !pane.sessionId}
    <div class="connect">
      <h2>{#if Icon}<Icon size={18} />{/if} {pane.host.name}</h2>
      <p class="dim">{pane.host.user}@{pane.host.hostname}:{pane.host.port}</p>
      <form onsubmit={(e) => { e.preventDefault(); connect() }}>
        <!-- svelte-ignore a11y_autofocus -->
        <input type="password" placeholder="password" bind:value={password} autofocus />
        <button class="primary" type="submit">Connect</button>
      </form>
      {#if pane.error}<div class="err">{pane.error}</div>{/if}
    </div>
  {:else}
    <div class="wrap">
      <div class="term"><Terminal id={pane.sessionId} /></div>
      {#if overlay}
        <div class="cover">
          <h3>{#if Icon}<Icon size={18} />{/if} {pane.host.name}</h3>
          <Stepper phase={pane.phase} error={pane.error} />
          {#if pane.phase === 'disconnected'}
            <button class="primary" onclick={reconnect}>Reconnect</button>
          {:else if pane.phase === 'error'}
            <button class="primary" onclick={reset}>Try again</button>
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
    border: 1px solid #222;
    display: flex;
    flex-direction: column;
  }
  .pane.active {
    border-color: #2b6cff;
  }
  .wrap {
    position: relative;
    flex: 1;
    min-height: 0;
  }
  .term {
    height: 100%;
  }
  .cover {
    position: absolute;
    inset: 0;
    background: rgba(10, 10, 10, 0.92);
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
    padding: 0.5rem;
    background: #1a1a1a;
    border: 1px solid #333;
    color: #eee;
    border-radius: 4px;
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
    padding: 0.5rem;
    background: #1a1a1a;
    border: 1px solid #2a2a2a;
    color: #eee;
    border-radius: 4px;
    margin-bottom: 0.3rem;
    cursor: pointer;
  }
  .pick button:hover {
    background: #202632;
  }
  .hicon {
    display: flex;
  }
  .dim {
    color: #888;
    font-size: 0.8rem;
  }
  .err {
    color: #f66;
    margin-top: 0.6rem;
  }
  button.primary {
    background: #2b6cff;
    border: 1px solid #2b6cff;
    color: #fff;
    padding: 0.5rem 0.9rem;
    border-radius: 4px;
    cursor: pointer;
  }
</style>
