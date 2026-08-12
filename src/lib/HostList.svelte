<script lang="ts">
  import { onMount, tick } from 'svelte'
  import { Search, Plus, Star, Pencil, Trash2, Rows3, Rows4, LayoutGrid, Upload, Download, X, FolderOpen } from '@lucide/svelte'
  import HostModal from './HostModal.svelte'
  import ImportExport from './ImportExport.svelte'
  import Sftp from './Sftp.svelte'
  import {
    store,
    ui,
    deleteHost,
    hostIcon,
    blankHost,
    tabHost,
    viewsStore,
    saveView,
    deleteView,
    inGroup,
    broadcast,
    type Host,
    type View,
  } from './state.svelte'
  import { toast } from './toast.svelte'
  import { roving } from './actions/roving'
  import { promptDialog } from './dialog.svelte'

  let { onopen }: { onopen: (h: Host) => void } = $props()

  let filter = $state('')
  let activeTags = $state<string[]>([]) // hosts must carry every active tag
  let editing = $state<Host | null>(null)
  let sftpHost = $state<Host | null>(null)
  let portio = $state<'import' | 'export' | null>(null)
  // View density persists across sessions (like the theme prefs).
  let mode = $state<'comfortable' | 'compact' | 'grid'>(
    (localStorage.getItem('ssh.hostView') as 'comfortable' | 'compact' | 'grid') || 'comfortable',
  )
  function setMode(m: typeof mode) {
    mode = m
    localStorage.setItem('ssh.hostView', m)
  }
  let searchEl = $state<HTMLInputElement>()

  const q = $derived(filter.toLowerCase().trim())
  // Distinct tags across all hosts, for the filter chips.
  const allTags = $derived([...new Set(store.hosts.flatMap((h) => h.tags))].sort())

  function match(h: Host): boolean {
    if (ui.group && !inGroup(h, ui.group)) return false
    if (activeTags.length && !activeTags.every((t) => h.tags.includes(t))) return false
    if (!q) return true
    return (
      h.name.toLowerCase().includes(q) ||
      h.hostname.toLowerCase().includes(q) ||
      h.tags.some((t) => t.toLowerCase().includes(q))
    )
  }

  function toggleTag(t: string) {
    activeTags = activeTags.includes(t) ? activeTags.filter((x) => x !== t) : [...activeTags, t]
  }

  function applyView(v: View) {
    filter = v.search
    activeTags = [...v.tags]
  }
  async function saveCurrentView() {
    const name = (await promptDialog({ title: 'Save view as', placeholder: 'View name' }))?.trim()
    if (name) {
      saveView({ id: crypto.randomUUID(), name, tags: [...activeTags], search: filter })
      toast(`View "${name}" saved`)
    }
  }

  const filtered = $derived(store.hosts.filter(match))
  const favorites = $derived(filtered.filter((h) => h.favorite))
  // When searching we show one flat list; otherwise Favorites + the rest.
  const rest = $derived(q ? filtered : filtered.filter((h) => !h.favorite))
  // Flat display order — the target of arrow-key selection.
  const visible = $derived(q ? filtered : [...favorites, ...rest])

  // Live sessions for the "Active now" strip — one card per open tab.
  const live = $derived(
    ui.tabs
      .map((t) => ({ host: tabHost(t), phase: t.panes.find((p) => p.phase)?.phase ?? '', key: t.key }))
      .filter((x) => x.host),
  )

  function statusColor(phase: string): string {
    if (phase === 'connecting' || phase === 'authenticating') return 'hsl(var(--connecting))'
    if (phase === 'connected') return 'hsl(var(--primary))'
    if (phase === 'error' || phase === 'disconnected') return 'hsl(var(--destructive))'
    return 'hsl(var(--muted-foreground))'
  }
  const phaseText: Record<string, string> = {
    connecting: 'connecting…',
    authenticating: 'authenticating…',
    connected: 'connected',
    disconnected: 'disconnected',
    error: 'error',
  }

  function goTab(key: string) {
    ui.active = key
    tick().then(() => window.dispatchEvent(new Event('resize')))
  }

  // Roving arrow-nav lives on the list container; Enter/click opens. This handler
  // only adds the '/' quick-focus for search, and only on the home surface.
  onMount(() => {
    const onKey = (e: KeyboardEvent) => {
      if (ui.active !== 'home' || editing || broadcast.on) return
      const t = e.target as HTMLElement | null
      if (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA')) return
      if (e.key === '/') {
        e.preventDefault()
        searchEl?.focus()
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  })
</script>

<div class="toolbar">
  <div class="seg"><button class="active">All Hosts</button></div>
  <div class="search">
    <Search size={14} color="hsl(var(--muted-foreground))" />
    <input bind:this={searchEl} bind:value={filter} placeholder="Search hosts" />
    {#if !filter}<kbd class="slash">/</kbd>{/if}
  </div>
  <div class="spacer"></div>
  <div class="seg density">
    <button class:active={mode === 'comfortable'} title="Comfortable" aria-label="Comfortable" onclick={() => setMode('comfortable')}>
      <Rows3 size={14} />
    </button>
    <button class:active={mode === 'compact'} title="Compact" aria-label="Compact" onclick={() => setMode('compact')}>
      <Rows4 size={14} />
    </button>
    <button class:active={mode === 'grid'} title="Grid" aria-label="Grid" onclick={() => setMode('grid')}>
      <LayoutGrid size={14} />
    </button>
  </div>
  <button class="btn" title="Import from ssh_config" onclick={() => (portio = 'import')}>
    <Upload size={14} /> Import
  </button>
  <button class="btn" title="Export to ssh_config" onclick={() => (portio = 'export')}>
    <Download size={14} /> Export
  </button>
  <button class="btn primary" onclick={() => (editing = blankHost())}>
    <Plus size={14} /> New Host
  </button>
</div>

{#if allTags.length || viewsStore.list.length}
  <div class="filterbar">
    {#each viewsStore.list as v (v.id)}
      <span class="viewchip">
        <button onclick={() => applyView(v)}>{v.name}</button>
        <button class="vx" aria-label="Delete view" onclick={() => { deleteView(v.id); toast('View removed') }}><X size={11} /></button>
      </span>
    {/each}
    {#each allTags as t}
      <button class="tagchip" class:on={activeTags.includes(t)} onclick={() => toggleTag(t)}>{t}</button>
    {/each}
    {#if activeTags.length || filter}
      <button class="fbtn" onclick={saveCurrentView}>Save view</button>
      <button class="fbtn" onclick={() => { activeTags = []; filter = '' }}>Clear</button>
    {/if}
  </div>
{/if}

{#if live.length}
  <div class="activebar">
    {#each live as l (l.key)}
      <button class="acard" onclick={() => goTab(l.key)}>
        <div class="top"><span class="dot" style:background={statusColor(l.phase)}></span> {l.host?.name}</div>
        <div class="meta mono">{phaseText[l.phase] ?? 'idle'}</div>
      </button>
    {/each}
  </div>
{/if}

<div
  class="listscroll"
  class:dense={mode === 'compact'}
  class:gridmode={mode === 'grid'}
  role="group"
  aria-label="Hosts"
  use:roving={{ orientation: mode === 'grid' ? 'grid' : 'vertical' }}
>
  {#if visible.length === 0}
    <div class="empty">
      <h2>No hosts yet</h2>
      <p class="muted">Add your first host to get started.</p>
      <button class="btn primary" onclick={() => (editing = blankHost())}><Plus size={14} /> Add a host</button>
    </div>
  {:else}
    {#if !q && favorites.length}
      <div class="cluster-h"><Star size={12} color="hsl(var(--amber))" /> Favorites</div>
      <div class="items">
        {#each favorites as h (h.id)}
          {@render item(h, visible.indexOf(h))}
        {/each}
      </div>
    {/if}
    {#if rest.length}
      <div class="cluster-h">{q ? `${filtered.length} match${filtered.length === 1 ? '' : 'es'}` : `All Hosts · ${store.hosts.length}`}</div>
      <div class="items">
        {#each rest as h (h.id)}
          {@render item(h, visible.indexOf(h))}
        {/each}
      </div>
    {/if}
  {/if}
</div>

{#snippet item(h: Host, i: number)}
  {#if mode === 'grid'}{@render card(h, i)}{:else}{@render row(h, i)}{/if}
{/snippet}

{#snippet card(h: Host, _i: number)}
  {@const Icon = hostIcon(h)}
  <div
    class="card"
    role="button"
    tabindex="-1"
    data-roving-item
    aria-label={`Connect to ${h.name}`}
    onclick={() => onopen(h)}
    onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onopen(h) } }}
  >
    {#if h.color}<span class="rail" style:background={h.color}></span>{/if}
    <div class="chead">
      <span class="hico" style:color={h.color ?? undefined}><Icon size={16} /></span>
      <span class="name">{h.name}</span>
      <span class="spacer"></span>
      {#if h.favorite}<span class="star"><Star size={13} /></span>{/if}
    </div>
    <div class="addr muted mono">{h.user}@{h.hostname}:{h.port}</div>
    <div class="ctags">
      {#each h.tags as t}<span class="chip tag">{t}</span>{/each}
    </div>
    <span class="actions">
      <button class="iconbtn" data-roving-action title="SFTP" aria-label={`SFTP to ${h.name}`} onclick={(e) => { e.stopPropagation(); sftpHost = h }}>
        <FolderOpen size={14} />
      </button>
      <button class="iconbtn" data-roving-action title="Edit" aria-label={`Edit ${h.name}`} onclick={(e) => { e.stopPropagation(); editing = { ...h } }}>
        <Pencil size={14} />
      </button>
      <button class="iconbtn" data-roving-action title="Delete" aria-label={`Delete ${h.name}`} onclick={(e) => { e.stopPropagation(); deleteHost(h.id); toast(`Deleted "${h.name}"`) }}>
        <Trash2 size={14} />
      </button>
    </span>
  </div>
{/snippet}

{#snippet row(h: Host, _i: number)}
  {@const Icon = hostIcon(h)}
  <div
    class="row"
    role="button"
    tabindex="-1"
    data-roving-item
    aria-label={`Connect to ${h.name}`}
    onclick={() => onopen(h)}
    onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onopen(h) } }}
  >
    {#if h.color}<span class="rail" style:background={h.color}></span>{/if}
    <span class="hico" style:color={h.color ?? undefined}><Icon size={16} /></span>
    <span class="name">{h.name}</span>
    <span class="addr muted mono">{h.user}@{h.hostname}:{h.port}</span>
    {#each h.tags as t}<span class="chip tag">{t}</span>{/each}
    <span class="spacer"></span>
    <span class="actions">
      <button class="iconbtn" data-roving-action title="SFTP" aria-label={`SFTP to ${h.name}`} onclick={(e) => { e.stopPropagation(); sftpHost = h }}>
        <FolderOpen size={14} />
      </button>
      <button class="iconbtn" data-roving-action title="Edit" aria-label={`Edit ${h.name}`} onclick={(e) => { e.stopPropagation(); editing = { ...h } }}>
        <Pencil size={14} />
      </button>
      <button class="iconbtn" data-roving-action title="Delete" aria-label={`Delete ${h.name}`} onclick={(e) => { e.stopPropagation(); deleteHost(h.id); toast(`Deleted "${h.name}"`) }}>
        <Trash2 size={14} />
      </button>
    </span>
    {#if h.favorite}<span class="star"><Star size={14} /></span>{/if}
  </div>
{/snippet}

{#if editing}
  <HostModal host={editing} onclose={() => (editing = null)} />
{/if}
{#if sftpHost}
  <Sftp host={sftpHost} onclose={() => (sftpHost = null)} />
{/if}
{#if portio}
  <ImportExport mode={portio} onclose={() => (portio = null)} />
{/if}

<style>
  .toolbar {
    height: 46px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 12px;
    border-bottom: 1px solid hsl(var(--border));
  }
  .filterbar {
    flex: none;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    border-bottom: 1px solid hsl(var(--border));
  }
  .tagchip,
  .fbtn {
    padding: 3px 10px;
    border: 1px solid hsl(var(--border));
    border-radius: 999px;
    background: hsl(var(--card));
    color: hsl(var(--muted-foreground));
    font: inherit;
    font-size: 11.5px;
    cursor: pointer;
  }
  .tagchip:hover,
  .fbtn:hover {
    background: hsl(var(--muted));
  }
  .tagchip.on {
    background: hsl(var(--primary) / 0.15);
    border-color: hsl(var(--primary) / 0.5);
    color: hsl(var(--foreground));
  }
  .viewchip {
    display: inline-flex;
    align-items: center;
    border: 1px solid hsl(var(--primary) / 0.4);
    background: hsl(var(--primary) / 0.1);
    border-radius: 999px;
    overflow: hidden;
  }
  .viewchip button {
    border: none;
    background: none;
    color: hsl(var(--foreground));
    font: inherit;
    font-size: 11.5px;
    padding: 3px 6px 3px 10px;
    cursor: pointer;
  }
  .viewchip .vx {
    display: grid;
    place-items: center;
    padding: 0 7px 0 2px;
    color: hsl(var(--muted-foreground));
  }
  .viewchip .vx:hover {
    color: hsl(var(--destructive));
  }
  .seg {
    display: flex;
    background: hsl(var(--muted));
    border-radius: 7px;
    padding: 2px;
  }
  .seg button {
    display: flex;
    align-items: center;
    padding: 4px 11px;
    border: none;
    background: none;
    border-radius: 5px;
    color: hsl(var(--muted-foreground));
    font-size: 12px;
    font-family: inherit;
    cursor: pointer;
  }
  .seg.density button {
    padding: 4px 8px;
  }
  .seg button.active {
    background: hsl(var(--card));
    color: hsl(var(--foreground));
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.25);
  }
  .search {
    flex: 1;
    max-width: 300px;
    display: flex;
    align-items: center;
    gap: 8px;
    background: hsl(var(--muted));
    border: 1px solid transparent;
    border-radius: 7px;
    padding: 3px 8px 3px 10px;
  }
  .search:focus-within {
    border-color: hsl(var(--ring) / 0.5);
  }
  .search input {
    flex: 1;
    min-width: 0;
    background: none;
    border: none;
    outline: none;
    padding: 0;
    color: inherit;
    font-size: 12.5px;
    font-family: inherit;
  }
  .search .slash {
    flex: none;
    font-family: ui-monospace, monospace;
    font-size: 11px;
    color: hsl(var(--muted-foreground));
    border: 1px solid hsl(var(--border));
    border-radius: 4px;
    padding: 0 5px;
    line-height: 16px;
  }
  .spacer {
    flex: 1;
  }
  .btn {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 7px 12px;
    border: none;
    border-radius: 7px;
    background: hsl(var(--muted));
    color: inherit;
    font-size: 12.5px;
    font-family: inherit;
    cursor: pointer;
  }
  .btn:hover {
    background: hsl(var(--border));
  }
  .btn.primary {
    background: hsl(var(--primary));
    color: hsl(var(--primary-foreground));
    font-weight: 600;
  }
  .btn.primary:hover {
    filter: brightness(1.08);
  }

  /* ---- active now ---- */
  .activebar {
    flex: none;
    display: flex;
    gap: 8px;
    padding: 10px 12px;
    border-bottom: 1px solid hsl(var(--border));
    overflow-x: auto;
  }
  .acard {
    min-width: 168px;
    text-align: left;
    background: hsl(var(--muted));
    border: none;
    border-radius: 8px;
    padding: 9px 11px;
    color: inherit;
    font-family: inherit;
    cursor: pointer;
  }
  .acard:hover {
    background: hsl(var(--border));
  }
  .acard .top {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12.5px;
  }
  .acard .meta {
    font-size: 11px;
    color: hsl(var(--muted-foreground));
    margin-top: 4px;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex: none;
  }

  /* ---- list ---- */
  .listscroll {
    flex: 1;
    overflow: auto;
    padding: 6px 0;
  }
  .cluster-h {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px 5px;
    font-size: 10.5px;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: hsl(var(--muted-foreground));
  }
  /* In row modes the wrapper is inert; in grid mode it lays the cards out. */
  .items {
    display: contents;
  }
  .gridmode .items {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(210px, 1fr));
    gap: 8px;
    padding: 4px 16px 8px;
  }
  .card {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 5px;
    padding: 11px 12px;
    border: 1px solid hsl(var(--border));
    border-radius: 9px;
    background: hsl(var(--card));
    cursor: pointer;
    overflow: hidden;
  }
  .card:hover {
    background: hsl(var(--muted));
  }
  .card:focus-visible {
    border-color: hsl(var(--primary) / 0.6);
    background: hsl(var(--primary) / 0.09);
    outline: none;
  }
  .chead {
    display: flex;
    align-items: center;
    gap: 9px;
  }
  .ctags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    min-height: 19px;
  }
  /* Always laid out, only toggled visible — keeps card height stable on hover. */
  .card .actions {
    display: flex;
    justify-content: flex-end;
    margin-top: 2px;
    visibility: hidden;
  }
  .card:hover .actions,
  .card:focus-within .actions {
    visibility: visible;
  }
  .card .rail {
    top: 0;
    bottom: 0;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 11px;
    height: 34px;
    padding: 0 16px;
    position: relative;
    cursor: pointer;
  }
  .dense .row {
    height: 28px;
  }
  .row:hover {
    background: hsl(var(--muted));
  }
  .row:focus-visible {
    background: hsl(var(--primary) / 0.09);
    box-shadow: inset 2px 0 0 hsl(var(--primary));
    outline: none;
  }
  /* host accent color — a subtle identity rail, distinct from the selection */
  .rail {
    position: absolute;
    left: 0;
    top: 6px;
    bottom: 6px;
    width: 2px;
    border-radius: 2px;
  }
  .hico {
    display: grid;
    place-items: center;
    color: hsl(var(--muted-foreground));
  }
  .name {
    font-weight: 500;
    font-size: 13px;
  }
  .addr {
    font-size: 12px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    padding: 2px 8px;
    border-radius: 5px;
    font-size: 11px;
    color: hsl(var(--muted-foreground));
  }
  .chip.tag {
    background: transparent;
    border: 1px solid hsl(var(--border));
  }
  .actions {
    display: none;
    align-items: center;
    gap: 1px;
  }
  .row:hover .actions,
  .row:focus-within .actions {
    display: flex;
  }
  .iconbtn {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border: none;
    background: none;
    border-radius: 6px;
    color: hsl(var(--muted-foreground));
    cursor: pointer;
  }
  .iconbtn:hover {
    background: hsl(var(--border));
    color: hsl(var(--foreground));
  }
  .star {
    display: grid;
    place-items: center;
    color: hsl(var(--amber));
  }

  /* ---- empty ---- */
  .empty {
    margin: auto;
    padding: 12vh 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    text-align: center;
  }
  .empty h2 {
    font-size: 18px;
    font-weight: 600;
    margin: 0;
  }
  .empty p {
    margin: 0;
    font-size: 13px;
  }
</style>
