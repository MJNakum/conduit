<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { Terminal } from '@xterm/xterm'
  import { FitAddon } from '@xterm/addon-fit'
  import { invoke } from '@tauri-apps/api/core'
  import { listen, type UnlistenFn } from '@tauri-apps/api/event'
  import '@xterm/xterm/css/xterm.css'

  // The session id returned by ssh_connect; scopes every event to this pane.
  let { id }: { id: string } = $props()

  let container: HTMLDivElement
  let term: Terminal
  let fit: FitAddon
  const unlisten: UnlistenFn[] = []

  onMount(async () => {
    term = new Terminal({ fontFamily: 'monospace', fontSize: 13, cursorBlink: true })
    fit = new FitAddon()
    term.loadAddon(fit)
    term.open(container)
    fit.fit()

    // Server → terminal. Payload bytes arrive as a JSON number array.
    // ponytail: number-array over the event bridge is wasteful; switch to
    // base64/binary if throughput ever matters.
    unlisten.push(
      await listen<{ id: string; bytes: number[] }>('ssh://data', (e) => {
        if (e.payload.id === id) term.write(new Uint8Array(e.payload.bytes))
      }),
    )

    // Terminal → server.
    term.onData((data) => invoke('ssh_write', { id, data }))
    term.onResize(({ cols, rows }) => invoke('ssh_resize', { id, cols, rows }))

    // Push the real size now that xterm knows its dimensions.
    const onResize = () => {
      fit.fit()
      invoke('ssh_resize', { id, cols: term.cols, rows: term.rows })
    }
    window.addEventListener('resize', onResize)
    unlisten.push(() => window.removeEventListener('resize', onResize))
    onResize()
    term.focus()
  })

  onDestroy(() => {
    unlisten.forEach((fn) => fn())
    term?.dispose()
  })
</script>

<div class="term" bind:this={container}></div>

<style>
  .term {
    width: 100%;
    height: 100%;
    background: #000;
  }
</style>
