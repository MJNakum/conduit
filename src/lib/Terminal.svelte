<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte'
  import { Terminal } from '@xterm/xterm'
  import { FitAddon } from '@xterm/addon-fit'
  import { SearchAddon } from '@xterm/addon-search'
  import { X, ChevronUp, ChevronDown } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { listen, type UnlistenFn } from '@tauri-apps/api/event'
  import '@xterm/xterm/css/xterm.css'

  // The session id returned by ssh_connect; scopes every event to this pane.
  let { id }: { id: string } = $props()

  let container: HTMLDivElement
  let searchInput = $state<HTMLInputElement>()
  let term: Terminal
  let fit: FitAddon
  let search: SearchAddon
  let showSearch = $state(false)
  let query = $state('')
  const unlisten: UnlistenFn[] = []

  onMount(async () => {
    term = new Terminal({ fontFamily: 'monospace', fontSize: 13, cursorBlink: true })
    fit = new FitAddon()
    search = new SearchAddon()
    term.loadAddon(fit)
    term.loadAddon(search)
    term.open(container)
    fit.fit()

    // Cmd+F toggles this terminal's search bar. Intercepting via xterm's key
    // handler scopes it to the focused terminal (each pane has its own).
    term.attachCustomKeyEventHandler((e) => {
      if (e.type === 'keydown' && e.metaKey && e.key === 'f') {
        openSearch()
        return false
      }
      return true
    })

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

    // Push the real size now that xterm knows its dimensions. Skip while the
    // pane is hidden (background tab): fitting a zero-size element would shrink
    // the remote PTY. It refits when the tab is shown (App dispatches resize).
    const onResize = () => {
      if (!container.clientWidth || !container.clientHeight) return
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

  async function openSearch() {
    showSearch = true
    await tick()
    searchInput?.focus()
    searchInput?.select()
  }

  function closeSearch() {
    showSearch = false
    search.clearDecorations()
    term.focus()
  }

  function findNext() {
    if (query) search.findNext(query)
  }
  function findPrev() {
    if (query) search.findPrevious(query)
  }

  function onSearchKey(e: KeyboardEvent) {
    if (e.key === 'Enter') (e.shiftKey ? findPrev : findNext)()
    else if (e.key === 'Escape') closeSearch()
  }
</script>

<div class="host">
  <div class="term" bind:this={container}></div>
  {#if showSearch}
    <div class="search">
      <input
        bind:this={searchInput}
        bind:value={query}
        oninput={findNext}
        onkeydown={onSearchKey}
        placeholder="Find"
      />
      <button aria-label="Previous" title="Previous (Shift+Enter)" onclick={findPrev}>
        <ChevronUp size={14} />
      </button>
      <button aria-label="Next" title="Next (Enter)" onclick={findNext}>
        <ChevronDown size={14} />
      </button>
      <button aria-label="Close" title="Close (Esc)" onclick={closeSearch}>
        <X size={14} />
      </button>
    </div>
  {/if}
</div>

<style>
  .host {
    position: relative;
    width: 100%;
    height: 100%;
  }
  .term {
    width: 100%;
    height: 100%;
    background: #000;
  }
  .search {
    position: absolute;
    top: 6px;
    right: 6px;
    display: flex;
    align-items: center;
    gap: 0.2rem;
    background: #1c1c1c;
    border: 1px solid #333;
    border-radius: 6px;
    padding: 0.25rem;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
  }
  .search input {
    background: #111;
    border: 1px solid #333;
    color: #eee;
    border-radius: 4px;
    padding: 0.25rem 0.4rem;
    width: 160px;
  }
  .search button {
    display: flex;
    padding: 0.25rem;
    background: none;
    border: none;
    color: #bbb;
    cursor: pointer;
    border-radius: 4px;
  }
  .search button:hover {
    background: #2a2a2a;
    color: #fff;
  }
</style>
