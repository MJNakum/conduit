<script lang="ts">
  import { onMount, tick } from 'svelte'
  import { listen } from '@tauri-apps/api/event'
  import {
    Terminal as TerminalIcon,
    Server,
    KeyRound,
    Zap,
    ArrowRightLeft,
    History,
    ChevronRight,
    Lock,
    Plus,
    X,
  } from '@lucide/svelte'
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

  // Sidebar sections. Only Hosts is wired now — Keys/Snippets/Port Forwards/
  // History and the Groups tree are inert placeholders for their later phases
  // (see docs/mvp-plan.md), shown so the shell has correct proportions.
  const laterSections = [
    { label: 'Keys', icon: KeyRound },
    { label: 'Snippets', icon: Zap },
    { label: 'Port Forwards', icon: ArrowRightLeft },
    { label: 'History', icon: History },
  ]

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

  // Aggregate a tab's panes into one status-language color (dot): in-progress >
  // connected > failed > idle. Decoupled from the host's accent color.
  function tabDot(tab: Tab): string {
    const phases = tab.panes.map((p) => p.phase)
    if (phases.some((p) => p === 'connecting' || p === 'authenticating'))
      return 'hsl(var(--connecting))'
    if (phases.some((p) => p === 'connected')) return 'hsl(var(--primary))'
    if (phases.some((p) => p === 'error' || p === 'disconnected'))
      return 'hsl(var(--destructive))'
    return 'hsl(var(--muted-foreground))'
  }
</script>

<main>
  <!-- TAB BAR — dot = status only; the thin top rule is the host's accent color -->
  <nav class="tabbar">
    <button
      class="tab pinned"
      class:active={ui.active === 'home'}
      onclick={() => activate('home')}
    >
      <TerminalIcon size={15} /> <span>Sessions</span>
    </button>
    {#each ui.tabs as tab (tab.key)}
      {@const host = tabHost(tab)}
      {@const Icon = host ? hostIcon(host) : TerminalIcon}
      <button
        class="tab"
        class:active={ui.active === tab.key}
        onclick={() => activate(tab.key)}
      >
        {#if host?.color}<span class="accent" style:background={host.color}></span>{/if}
        <span class="pdot" style:background={tabDot(tab)}></span>
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
    <button class="plus" aria-label="New tab" onclick={() => activate('home')}>
      <Plus size={15} />
    </button>
  </nav>

  <div class="body">
    <!-- SIDEBAR — section switcher + vault pill (local-first trust anchor) -->
    <aside class="sidebar">
      <div class="side-scroll">
        <button
          class="side-item"
          class:active={ui.active === 'home'}
          onclick={() => activate('home')}
        >
          <Server size={15} /> Hosts
        </button>
        {#each laterSections as s}
          {@const SIcon = s.icon}
          <div class="side-item soon" aria-disabled="true" title="Coming in a later phase">
            <SIcon size={15} /> {s.label}
          </div>
        {/each}
        <div class="side-head">Groups</div>
        <div class="side-item soon" aria-disabled="true" title="Coming in a later phase">
          <ChevronRight size={15} /> Production
        </div>
        <div class="side-item soon" aria-disabled="true" title="Coming in a later phase">
          <ChevronRight size={15} /> Clients
        </div>
      </div>
      <div class="vault">
        <Lock size={14} color="hsl(var(--primary))" /> Vault unlocked
        <span class="mono kbd">⌘L</span>
      </div>
    </aside>

    <!-- MAIN -->
    <div class="main">
      <div class="page" class:hidden={ui.active !== 'home'}>
        <HostList onopen={open} />
      </div>
      {#each ui.tabs as tab (tab.key)}
        <div class="page" class:hidden={ui.active !== tab.key}>
          <TabView {tab} />
        </div>
      {/each}
    </div>
  </div>

  <!-- FOOTER — slim global status only (per-session liveness lives on tab dots) -->
  <footer>
    <span>{activeCount()} session{activeCount() === 1 ? '' : 's'} active</span>
    <span class="spacer"></span>
    <span>Theme: Dark</span>
    <span>·</span>
    <span class="mono">⌘K</span>
    <span>command palette</span>
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
    background: hsl(var(--background));
    color: hsl(var(--foreground));
  }

  /* ---- tab bar ---- */
  .tabbar {
    height: 38px;
    display: flex;
    align-items: flex-end;
    gap: 2px;
    padding: 0 8px;
    background: hsl(var(--background));
    border-bottom: 1px solid hsl(var(--border));
    overflow-x: auto;
  }
  .tab {
    height: 30px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 12px;
    border: 1px solid hsl(var(--border));
    border-bottom: none;
    border-radius: 8px 8px 0 0;
    background: hsl(var(--card));
    color: hsl(var(--muted-foreground));
    cursor: pointer;
    white-space: nowrap;
    font-size: 12.5px;
    position: relative;
  }
  .tab.active {
    background: hsl(var(--muted));
    color: hsl(var(--foreground));
  }
  .tab.pinned {
    background: transparent;
    border-color: transparent;
  }
  .tab.pinned.active {
    background: hsl(var(--muted));
  }
  /* accent = host color, a thin top rule (identity), never the status dot */
  .tab .accent {
    position: absolute;
    top: 0;
    left: 10px;
    right: 10px;
    height: 2px;
    border-radius: 2px;
  }
  .pdot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex: none;
  }
  .close {
    display: grid;
    place-items: center;
    width: 16px;
    height: 16px;
    border-radius: 4px;
    color: hsl(var(--muted-foreground));
    opacity: 0;
  }
  .tab:hover .close {
    opacity: 0.6;
  }
  .close:hover {
    opacity: 1;
    background: hsl(var(--border));
  }
  .plus {
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    border-radius: 6px;
    background: none;
    border: none;
    color: hsl(var(--muted-foreground));
    cursor: pointer;
  }
  .plus:hover {
    background: hsl(var(--muted));
  }

  /* ---- body / sidebar ---- */
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .sidebar {
    width: 236px;
    flex: none;
    background: hsl(var(--card));
    border-right: 1px solid hsl(var(--border));
    display: flex;
    flex-direction: column;
  }
  .side-scroll {
    flex: 1;
    overflow: auto;
    padding: 8px;
  }
  .side-item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 6px 9px;
    border: none;
    background: none;
    border-radius: 6px;
    color: hsl(var(--muted-foreground));
    font-size: 12.5px;
    font-family: inherit;
    text-align: left;
    cursor: pointer;
  }
  .side-item:hover {
    background: hsl(var(--muted));
  }
  .side-item.active {
    background: hsl(var(--primary) / 0.1);
    color: hsl(var(--foreground));
  }
  .side-item.soon {
    opacity: 0.45;
    cursor: default;
  }
  .side-item.soon:hover {
    background: none;
  }
  .side-head {
    font-size: 10px;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: hsl(var(--muted-foreground));
    padding: 14px 9px 5px;
    opacity: 0.65;
  }
  .vault {
    margin: 8px;
    padding: 8px 10px;
    border-radius: 8px;
    background: hsl(var(--muted));
    display: flex;
    align-items: center;
    gap: 9px;
    font-size: 12px;
  }
  .kbd {
    margin-left: auto;
    font-size: 10px;
    color: hsl(var(--muted-foreground));
  }

  /* ---- main content ---- */
  .main {
    flex: 1;
    min-width: 0;
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

  /* ---- footer ---- */
  footer {
    height: 26px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 12px;
    background: hsl(var(--card));
    border-top: 1px solid hsl(var(--border));
    font-size: 11.5px;
    color: hsl(var(--muted-foreground));
  }
  .spacer {
    flex: 1;
  }
</style>
