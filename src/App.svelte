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
    SlidersHorizontal,
    Plus,
    X,
  } from '@lucide/svelte'
  import HostList from './lib/HostList.svelte'
  import TabView from './lib/TabView.svelte'
  import Palette from './lib/Palette.svelte'
  import KeyManager from './lib/KeyManager.svelte'
  import PortForwards from './lib/PortForwards.svelte'
  import Snippets from './lib/Snippets.svelte'
  import HistoryView from './lib/History.svelte'
  import Settings from './lib/Settings.svelte'
  import Toaster from './lib/Toaster.svelte'
  import LockScreen from './lib/LockScreen.svelte'
  import BroadcastBar from './lib/BroadcastBar.svelte'
  import ShortcutsOverlay from './lib/ShortcutsOverlay.svelte'
  import DialogHost from './lib/DialogHost.svelte'
  import AuthPrompt from './lib/AuthPrompt.svelte'
  import Splash from './lib/Splash.svelte'
  import { Radio } from '@lucide/svelte'
  import { settings, applyAppTheme, setAppTheme, type AppTheme } from './lib/theme.svelte'
  import { matchEvent, bindingOf, formatBinding, isPrintableChord, type ActionId } from './lib/keymap.svelte'
  import { cycleRegion, noteFocus, region } from './lib/focus.svelte'
  import { roving } from './lib/actions/roving'
  import { vault, lockVault } from './lib/vault.svelte'
  import { toast } from './lib/toast.svelte'
  import { checkForUpdates } from './lib/updates.svelte'
  import { loadForwards, applyForwardState } from './lib/state.svelte'
  import {
    ui,
    broadcast,
    connectedSessions,
    loadHosts,
    loadKeys,
    loadSnippets,
    openTab,
    closeTab,
    applyState,
    applyLog,
    applyPrompt,
    activeCount,
    hostIcon,
    tabHost,
    groupNodes,
    type Host,
    type Tab,
    type StatePayload,
    type LogPayload,
    type PromptPayload,
  } from './lib/state.svelte'

  let showSplash = $state(true)
  let paletteOpen = $state(false)
  let helpOpen = $state(false)
  // Which home-view section is showing (only when no terminal tab is active).
  let section = $state<'hosts' | 'keys' | 'snippets' | 'forwards' | 'history' | 'settings'>('hosts')
  let settingsTab = $state<'appearance' | 'shortcuts' | 'about'>('appearance')

  onMount(() => {
    applyAppTheme()
    loadHosts()
    loadKeys()
    loadForwards()
    loadSnippets()
    // Silent auto-update check on launch; only surfaces if an update is found.
    checkForUpdates()
    listen<StatePayload>('ssh://state', (e) => applyState(e.payload))
    listen<LogPayload>('ssh://log', (e) => applyLog(e.payload))
    listen<PromptPayload>('ssh://prompt', (e) => applyPrompt(e.payload))
    listen<{ id: string; state: string; message?: string }>('forward://state', (e) =>
      applyForwardState(e.payload.id, e.payload.state, e.payload.message),
    )
    // Global shortcut dispatch. Bindings come from the customizable keymap;
    // captured at window level so they work over xterm too.
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && broadcast.on) {
        broadcast.on = false
        return
      }
      if (vault.locked) return // no shortcuts while locked
      // Never hijack a character key while a text field or terminal has focus —
      // `?` must type there, not open the shortcut sheet.
      if (isPrintableChord(e) && isTextField(document.activeElement)) return
      const id = matchEvent(e)
      if (id) {
        e.preventDefault()
        runAction(id)
      }
    }
    window.addEventListener('keydown', onKey)
    // Track which region focus lives in so Focus-Next-Part (F6) advances from it.
    const onFocusIn = (e: FocusEvent) => noteFocus(e.target)
    window.addEventListener('focusin', onFocusIn)
    return () => {
      window.removeEventListener('keydown', onKey)
      window.removeEventListener('focusin', onFocusIn)
    }
  })

  // A focused text-entry surface (native field, contentEditable, or the xterm
  // textarea) where character shortcuts must yield to typing.
  function isTextField(el: Element | null): boolean {
    if (!el) return false
    const tag = el.tagName
    return (
      tag === 'INPUT' ||
      tag === 'TEXTAREA' ||
      tag === 'SELECT' ||
      (el as HTMLElement).isContentEditable
    )
  }

  // Drop focus into the visible tab's terminal (xterm's helper textarea).
  function focusTerminal() {
    const el = document.querySelector<HTMLTextAreaElement>(
      '.page:not(.hidden) .xterm-helper-textarea',
    )
    el?.focus()
  }

  // Move between the Sessions tab and the open terminal tabs by offset.
  function switchTab(dir: 1 | -1) {
    const keys = ['home', ...ui.tabs.map((t) => t.key)]
    const i = keys.indexOf(ui.active)
    activate(keys[(i + dir + keys.length) % keys.length])
  }

  function cycleTheme() {
    const order: AppTheme[] = ['dark', 'light', 'system']
    const next = order[(order.indexOf(settings.appTheme) + 1) % order.length]
    setAppTheme(next)
    toast(`Theme: ${next}`)
  }

  function goSection(s: typeof section) {
    section = s
    if (s === 'hosts') ui.group = null
    activate('home')
  }

  function runAction(id: ActionId) {
    switch (id) {
      case 'palette': paletteOpen = !paletteOpen; break
      case 'settings': case 'gotoSettings': goSection('settings'); break
      case 'cycleTheme': cycleTheme(); break
      case 'newTab': activate('home'); break
      case 'closeTab': if (ui.active !== 'home') closeTab(ui.active); break
      case 'nextTab': switchTab(1); break
      case 'prevTab': switchTab(-1); break
      case 'lockVault': lockVault(); break
      case 'broadcast': if (connectedSessions().length) broadcast.on = !broadcast.on; break
      case 'gotoHosts': goSection('hosts'); break
      case 'gotoKeys': goSection('keys'); break
      case 'gotoSnippets': goSection('snippets'); break
      case 'gotoForwards': goSection('forwards'); break
      case 'gotoHistory': goSection('history'); break
      case 'cycleRegionNext': cycleRegion(1); break
      case 'cycleRegionPrev': cycleRegion(-1); break
      case 'focusTerminal': focusTerminal(); break
      case 'help': helpOpen = !helpOpen; break
    }
  }

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
  <nav class="tabbar" aria-label="Open sessions" use:region={'tabbar'} use:roving={{ orientation: 'horizontal' }}>
    <button
      class="tab pinned"
      class:active={ui.active === 'home'}
      data-roving-item
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
        data-roving-item
        onclick={() => activate(tab.key)}
        onkeydown={(e) => { if (e.key === 'Delete' || e.key === 'Backspace') { e.preventDefault(); closeTab(tab.key) } }}
      >
        {#if host?.color}<span class="accent" style:background={host.color}></span>{/if}
        <span class="pdot" style:background={tabDot(tab)}></span>
        <Icon size={15} />
        <span>{host?.name ?? 'New tab'}{tab.panes.length > 1 ? ` (${tab.panes.length})` : ''}</span>
        <span
          class="close"
          role="button"
          tabindex="-1"
          aria-label="Close tab"
          onclick={(e) => { e.stopPropagation(); closeTab(tab.key) }}
        ><X size={14} /></span>
      </button>
    {/each}
    <button class="plus" aria-label="New tab" data-roving-item onclick={() => activate('home')}>
      <Plus size={15} />
    </button>
  </nav>

  <div class="body">
    <!-- SIDEBAR — section switcher + vault pill (local-first trust anchor) -->
    <aside class="sidebar" use:region={'sidebar'}>
      <div class="side-scroll" role="navigation" aria-label="Sections" use:roving={{ orientation: 'vertical' }}>
        <button
          class="side-item"
          class:active={ui.active === 'home' && section === 'hosts' && ui.group === null}
          data-roving-item
          onclick={() => { section = 'hosts'; ui.group = null; activate('home') }}
        >
          <Server size={15} /> Hosts
        </button>
        <button
          class="side-item"
          class:active={ui.active === 'home' && section === 'keys'}
          data-roving-item
          onclick={() => { section = 'keys'; activate('home') }}
        >
          <KeyRound size={15} /> Keys
        </button>
        <button
          class="side-item"
          class:active={ui.active === 'home' && section === 'snippets'}
          data-roving-item
          onclick={() => { section = 'snippets'; activate('home') }}
        >
          <Zap size={15} /> Snippets
        </button>
        <button
          class="side-item"
          class:active={ui.active === 'home' && section === 'forwards'}
          data-roving-item
          onclick={() => { section = 'forwards'; activate('home') }}
        >
          <ArrowRightLeft size={15} /> Port Forwards
        </button>
        <button
          class="side-item"
          class:active={ui.active === 'home' && section === 'settings'}
          data-roving-item
          onclick={() => goSection('settings')}
        >
          <SlidersHorizontal size={15} /> Settings
        </button>
        <button
          class="side-item"
          class:active={ui.active === 'home' && section === 'history'}
          data-roving-item
          onclick={() => goSection('history')}
        >
          <History size={15} /> History
        </button>
        <div class="side-head">Groups</div>
        {#if groupNodes().length === 0}
          <div class="side-item soon" aria-disabled="true">Set a host's Group to build the tree</div>
        {:else}
          {#each groupNodes() as g (g.path)}
            <button
              class="side-item"
              style:padding-left={`${9 + g.depth * 14}px`}
              class:active={ui.active === 'home' && section === 'hosts' && ui.group === g.path}
              data-roving-item
              onclick={() => { section = 'hosts'; ui.group = g.path; activate('home') }}
            >
              <ChevronRight size={15} /> {g.label}
            </button>
          {/each}
        {/if}
      </div>
      <button class="vault" onclick={lockVault} title="Lock the vault">
        <Lock size={14} color="hsl(var(--primary))" /> Vault unlocked
        <span class="mono kbd">{formatBinding(bindingOf('lockVault'))}</span>
      </button>
    </aside>

    <!-- MAIN -->
    <div class="main" use:region={'content'}>
      <div class="page" class:hidden={ui.active !== 'home'}>
        {#if section === 'keys'}
          <KeyManager />
        {:else if section === 'snippets'}
          <Snippets />
        {:else if section === 'forwards'}
          <PortForwards />
        {:else if section === 'history'}
          <HistoryView />
        {:else if section === 'settings'}
          <Settings bind:tab={settingsTab} />
        {:else}
          <HostList onopen={open} />
        {/if}
      </div>
      {#each ui.tabs as tab (tab.key)}
        <div class="page" class:hidden={ui.active !== tab.key}>
          <TabView {tab} />
        </div>
      {/each}
    </div>
  </div>

  {#if broadcast.on}
    <BroadcastBar />
  {/if}

  <!-- FOOTER — slim global status only (per-session liveness lives on tab dots) -->
  <footer use:region={'footer'} use:roving={{ orientation: 'horizontal' }}>
    <span>{activeCount()} session{activeCount() === 1 ? '' : 's'} active</span>
    <span class="spacer"></span>
    <button class="themebtn" class:bcast={broadcast.on} data-roving-item onclick={() => (broadcast.on = !broadcast.on)} disabled={connectedSessions().length === 0}>
      <Radio size={12} /> Broadcast
    </button>
    <span>·</span>
    <button class="themebtn" data-roving-item onclick={() => { settingsTab = 'appearance'; goSection('settings') }}>Theme: {settings.appTheme}</button>
    <span>·</span>
    <span class="mono">{formatBinding(bindingOf('palette'))}</span>
    <span>command palette</span>
  </footer>

  {#if paletteOpen}
    <Palette onclose={() => (paletteOpen = false)} onrun={runAction} onactivateTab={activate} />
  {/if}
  {#if helpOpen}
    <ShortcutsOverlay onclose={() => (helpOpen = false)} />
  {/if}
  {#if vault.locked}
    <LockScreen />
  {/if}
  <DialogHost />
  <AuthPrompt />
  <Toaster />
  {#if showSplash}
    <Splash done={() => (showSplash = false)} />
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
    border: none;
    width: calc(100% - 16px);
    color: inherit;
    font-family: inherit;
    display: flex;
    align-items: center;
    gap: 9px;
    font-size: 12px;
    cursor: pointer;
    text-align: left;
  }
  .vault:hover {
    background: hsl(var(--border));
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
  .themebtn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    background: none;
    border: none;
    color: hsl(var(--muted-foreground));
    font: inherit;
    font-size: 11.5px;
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 5px;
    text-transform: capitalize;
  }
  .themebtn:hover {
    background: hsl(var(--muted));
    color: hsl(var(--foreground));
  }
  .themebtn:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .themebtn.bcast {
    color: hsl(var(--amber));
    background: hsl(var(--amber) / 0.15);
  }
</style>
