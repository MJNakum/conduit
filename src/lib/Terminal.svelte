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
  // theme/font come resolved from the pane (per-connection or global default).
  let {
    id,
    theme,
    fontFamily = 'monospace',
    fontSize = 13,
  }: {
    id: string
    theme?: Record<string, string>
    fontFamily?: string
    fontSize?: number
  } = $props()

  let container: HTMLDivElement
  let searchInput = $state<HTMLInputElement>()
  let term: Terminal
  let fit: FitAddon
  let search: SearchAddon
  let showSearch = $state(false)
  let query = $state('')
  const unlisten: UnlistenFn[] = []

  onMount(async () => {
    term = new Terminal({ fontFamily, fontSize, cursorBlink: true, theme })
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

  // Live-apply theme/font changes (global toggle or per-connection edit).
  $effect(() => {
    if (!term) return
    term.options.theme = theme
    term.options.fontFamily = fontFamily
    term.options.fontSize = fontSize
    fit?.fit()
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
  <div class="term" bind:this={container} style:background={theme?.background ?? '#0a0e13'}></div>
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
    background: #0a0e13;
  }
  .search {
    position: absolute;
    top: 8px;
    right: 12px;
    display: flex;
    align-items: center;
    gap: 6px;
    background: hsl(var(--popover));
    border: 1px solid hsl(var(--border));
    border-radius: 8px;
    padding: 6px 8px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
  }
  .search input {
    background: hsl(var(--muted));
    border: none;
    outline: none;
    color: hsl(var(--foreground));
    border-radius: 5px;
    padding: 4px 8px;
    width: 150px;
    font-size: 12px;
    font-family: inherit;
  }
  .search button {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    padding: 0;
    background: none;
    border: none;
    color: hsl(var(--muted-foreground));
    cursor: pointer;
    border-radius: 5px;
  }
  .search button:hover {
    background: hsl(var(--muted));
    color: hsl(var(--foreground));
  }
</style>
