<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import Terminal from './lib/Terminal.svelte'

  // Phase 0: a single hardcoded-ish connection. No persistence, no Keychain —
  // host list, tabs and saved hosts are Phase 1.
  let host = $state('localhost')
  let port = $state(22)
  let user = $state('')
  let password = $state('')

  let sessionId = $state<string | null>(null)
  let phase = $state<string>('') // real state from ssh://state (stepper source)
  let error = $state<string>('')

  // Reflect the real connection pipeline — never a faked animation.
  listen<{ id: string; state: string; message?: string; reason?: string }>(
    'ssh://state',
    (e) => {
      if (sessionId && e.payload.id !== sessionId) return
      phase = e.payload.state
      if (e.payload.state === 'error') error = e.payload.message ?? 'error'
    },
  )

  async function connect() {
    error = ''
    phase = ''
    try {
      sessionId = await invoke<string>('ssh_connect', { host, port, user, password })
      password = '' // don't keep the secret in memory longer than needed
    } catch (e) {
      error = String(e)
    }
  }
</script>

<main>
  {#if sessionId}
    <div class="bar">
      <span>session {sessionId}</span>
      <span class="phase">{phase}</span>
      {#if error}<span class="err">{error}</span>{/if}
    </div>
    <div class="pane">
      <Terminal id={sessionId} />
    </div>
  {:else}
    <form onsubmit={(e) => { e.preventDefault(); connect() }}>
      <h1>SSH — Phase 0</h1>
      <input placeholder="host" bind:value={host} />
      <input placeholder="port" type="number" bind:value={port} />
      <input placeholder="user" bind:value={user} />
      <input placeholder="password" type="password" bind:value={password} />
      <button type="submit">Connect</button>
      {#if phase}<div class="phase">{phase}</div>{/if}
      {#if error}<div class="err">{error}</div>{/if}
    </form>
  {/if}
</main>

<style>
  main {
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: #111;
    color: #eee;
    font-family: system-ui, sans-serif;
  }
  form {
    margin: auto;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    width: 260px;
  }
  input,
  button {
    padding: 0.5rem;
    font-size: 0.95rem;
  }
  .bar {
    display: flex;
    gap: 1rem;
    padding: 0.4rem 0.8rem;
    background: #1c1c1c;
    font-size: 0.8rem;
  }
  .pane {
    flex: 1;
    min-height: 0;
  }
  .phase {
    color: #6cf;
  }
  .err {
    color: #f66;
  }
</style>
