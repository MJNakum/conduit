<script lang="ts">
  import { onMount, tick } from 'svelte'
  import { listen } from '@tauri-apps/api/event'
  import { LayoutGrid, X } from '@lucide/svelte'
  import HostList from './lib/HostList.svelte'
  import TabView from './lib/TabView.svelte'
  import Palette from './lib/Palette.svelte'
  import {
    ui,
    loadHosts,
    openTab,
    closeTab,
    applyState,
    activeCount,
    hostIcon,
    tabHost,
    type Host,
    type Tab,
  } from './lib/state.svelte'

  let paletteOpen = $state(false)

  onMount(() => {
    loadHosts()
    listen<{ id: string; state: string; message?: string }>('ssh://state', (e) =>
      applyState(e.payload.id, e.payload.state, e.payload.message),
    )
    // Cmd+K toggles the command palette. Captured at window level (works over
    // xterm too, since it fires before the terminal sees the key).
    const onKey = (e: KeyboardEvent) => {
      if (e.metaKey && e.key === 'k') {
        e.preventDefault()
        paletteOpen = !paletteOpen
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  })

  async function open(h: Host) {
    openTab(h)
    await activate(ui.active)
  }

  // Switch tabs, then nudge the now-visible terminal to refit its (previously
  // hidden, zero-size) container.
  async function activate(key: string) {
    ui.active = key
    await tick()
    window.dispatchEvent(new Event('resize'))
  }

  // Aggregate a tab's panes into one status color: in-progress > connected >
  // failed > idle.
  function tabColor(tab: Tab): string {
    const phases = tab.panes.map((p) => p.phase)
    if (phases.some((p) => p === 'connecting' || p === 'authenticating')) return '#6cf'
    if (phases.some((p) => p === 'connected')) return '#3c3'
    if (phases.some((p) => p === 'error' || p === 'disconnected')) return '#a33'
    return '#555'
  }
</script>

<main>
  <nav class="tabs">
    <button class="tab" class:active={ui.active === 'home'} onclick={() => activate('home')}>
      <LayoutGrid size={15} /> All Sessions
    </button>
    {#each ui.tabs as tab (tab.key)}
      {@const host = tabHost(tab)}
      {@const Icon = host ? hostIcon(host) : LayoutGrid}
      <button
        class="tab"
        class:active={ui.active === tab.key}
        style:border-bottom-color={host?.color ?? undefined}
        onclick={() => activate(tab.key)}
      >
        <span class="pdot" style:background={tabColor(tab)}></span>
        <Icon size={15} />
        <span>{host?.name ?? 'New tab'}{tab.panes.length > 1 ? ` (${tab.panes.length})` : ''}</span>
        <span
          class="close"
          role="button"
          tabindex="0"
          aria-label="Close tab"
          onclick={(e) => { e.stopPropagation(); closeTab(tab.key) }}
          onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); closeTab(tab.key) } }}
        ><X size={14} /></span>
      </button>
    {/each}
  </nav>

  <div class="body">
    <div class="page" class:hidden={ui.active !== 'home'}>
      <HostList onopen={open} />
    </div>
    {#each ui.tabs as tab (tab.key)}
      <div class="page" class:hidden={ui.active !== tab.key}>
        <TabView {tab} />
      </div>
    {/each}
  </div>

  <footer>
    <span>{activeCount()} active</span>
    {#if ui.tabs.length}<span class="dim">· {ui.tabs.length} tab{ui.tabs.length > 1 ? 's' : ''}</span>{/if}
    <span class="hint">⌘K palette · ⌘F find</span>
  </footer>

  {#if paletteOpen}
    <Palette onclose={() => (paletteOpen = false)} />
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
  .tabs {
    display: flex;
    background: #0c0c0c;
    border-bottom: 1px solid #222;
    overflow-x: auto;
  }
  .tab {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.55rem 0.9rem;
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: #bbb;
    cursor: pointer;
    white-space: nowrap;
    font-size: 0.85rem;
  }
  .tab.active {
    color: #fff;
    background: #161616;
    border-bottom-color: #2b6cff;
  }
  .pdot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }
  .close {
    color: #777;
    font-size: 1rem;
    line-height: 1;
  }
  .close:hover {
    color: #f66;
  }
  .body {
    flex: 1;
    min-height: 0;
    position: relative;
  }
  .page {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .page.hidden {
    display: none;
  }
  footer {
    padding: 0.3rem 0.9rem;
    background: #0c0c0c;
    border-top: 1px solid #222;
    font-size: 0.75rem;
    color: #9c9;
  }
  .dim {
    color: #777;
  }
  footer {
    display: flex;
    gap: 0.6rem;
    align-items: center;
  }
  .hint {
    margin-left: auto;
    color: #666;
  }
</style>
