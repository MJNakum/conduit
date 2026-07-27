<script lang="ts">
  import { tick } from 'svelte'
  import { Search, Command, TerminalSquare, type Icon } from '@lucide/svelte'
  import { store, ui, openTab, hostIcon, tabHost, fuzzy } from './state.svelte'
  import { trapFocus } from './actions/trapFocus'
  import { ACTIONS, bindingOf, formatBinding, type ActionId } from './keymap.svelte'

  let {
    onclose,
    onrun,
    onactivateTab,
  }: { onclose: () => void; onrun: (id: ActionId) => void; onactivateTab: (key: string) => void } = $props()

  let query = $state('')
  let sel = $state(0)
  let input = $state<HTMLInputElement>()

  // Group names double as section headers, in this display order.
  type Kind = 'Hosts' | 'Open Tabs' | 'Navigation' | 'Tabs' | 'Session' | 'General'
  const ORDER: Kind[] = ['Hosts', 'Open Tabs', 'Navigation', 'Tabs', 'Session', 'General']
  type Item = { label: string; sub: string; icon: typeof Icon; kind: Kind; hint?: string[]; run: () => void }

  // A universal hub: connect to any host, jump to any open tab, and run every
  // registered action (each shown with its live binding).
  const items = $derived.by<Item[]>(() => {
    const list: Item[] = []

    // Hosts — open in a new tab.
    for (const h of store.hosts) {
      list.push({
        label: h.name,
        sub: `${h.user}@${h.hostname}:${h.port}`,
        icon: hostIcon(h),
        kind: 'Hosts',
        hint: ['↵ connect'],
        run: () => {
          openTab(h)
          onclose()
        },
      })
    }

    // Open tabs — switch focus to a live session.
    for (const t of ui.tabs) {
      if (t.key === ui.active) continue
      list.push({
        label: `Switch to ${tabHost(t)?.name ?? 'New tab'}`,
        sub: 'open session',
        icon: TerminalSquare,
        kind: 'Open Tabs',
        run: () => {
          onactivateTab(t.key)
          onclose()
        },
      })
    }

    // Every registered action except opening this palette itself.
    for (const a of ACTIONS) {
      if (a.id === 'palette') continue
      list.push({
        label: a.label,
        sub: '',
        icon: Command,
        kind: a.category as Kind,
        hint: [formatBinding(bindingOf(a.id))],
        run: () => {
          onclose()
          onrun(a.id)
        },
      })
    }
    return list
  })

  const filtered = $derived(
    items
      .map((it) => ({ it, score: fuzzy(query, `${it.label} ${it.sub}`) }))
      .filter((x) => x.score >= 0)
      .sort((a, b) => b.score - a.score)
      .map((x) => x.it),
  )

  // Group into ordered sections while keeping a flat index for keyboard nav.
  const groups = $derived(
    ORDER.map((kind) => ({ kind, items: filtered.filter((it) => it.kind === kind) })).filter(
      (g) => g.items.length,
    ),
  )

  $effect(() => {
    if (sel >= filtered.length) sel = Math.max(0, filtered.length - 1)
  })
  $effect(() => {
    tick().then(() => input?.focus())
  })

  function onKey(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      sel = Math.min(sel + 1, filtered.length - 1)
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      sel = Math.max(sel - 1, 0)
    } else if (e.key === 'Enter') {
      e.preventDefault()
      filtered[sel]?.run()
    } else if (e.key === 'Escape') {
      onclose()
    }
  }
</script>

<div class="backdrop" onclick={onclose} role="presentation">
  <div class="palette" onclick={(e) => e.stopPropagation()} role="dialog" tabindex="-1" aria-modal="true" use:trapFocus={{ onclose }}>
    <div class="pin">
      <Search size={16} color="hsl(var(--muted-foreground))" />
      <input bind:this={input} bind:value={query} onkeydown={onKey} placeholder="Search hosts and actions…" />
      <span class="kbd">esc</span>
    </div>
    <div class="results">
      {#each groups as g (g.kind)}
        <div class="pgroup">{g.kind}</div>
        {#each g.items as item (item.label + item.sub)}
          {@const i = filtered.indexOf(item)}
          {@const ItemIcon = item.icon}
          <button class="presult" class:on={i === sel} onclick={item.run} onmouseenter={() => (sel = i)}>
            <span class="txt">
              <ItemIcon size={16} color="hsl(var(--muted-foreground))" />
              <span class="label">{item.label}</span>
              {#if item.sub}<span class="muted mono sub">{item.sub}</span>{/if}
            </span>
            {#if item.hint}
              <span class="k">{#each item.hint as h}<span class="kbd">{h}</span>{/each}</span>
            {/if}
          </button>
        {/each}
      {:else}
        <div class="none">No matches</div>
      {/each}
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 12vh;
    z-index: 60;
  }
  .palette {
    width: 560px;
    max-width: 90vw;
    background: hsl(var(--popover));
    border: 1px solid hsl(var(--border));
    border-radius: 12px;
    overflow: hidden;
    box-shadow: 0 24px 64px rgba(0, 0, 0, 0.55);
  }
  .pin {
    display: flex;
    align-items: center;
    gap: 11px;
    padding: 14px 16px;
    border-bottom: 1px solid hsl(var(--border));
  }
  .pin input {
    flex: 1;
    background: none;
    border: none;
    outline: none;
    color: inherit;
    font-size: 15px;
    font-family: inherit;
  }
  .results {
    padding: 4px 0 8px;
    max-height: 52vh;
    overflow-y: auto;
  }
  .pgroup {
    padding: 9px 16px 3px;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: hsl(var(--muted-foreground));
  }
  .presult {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 11px;
    padding: 9px 16px;
    background: none;
    border: none;
    color: inherit;
    font-size: 13px;
    font-family: inherit;
    text-align: left;
    cursor: pointer;
  }
  .presult.on {
    background: hsl(var(--primary) / 0.1);
    box-shadow: inset 2px 0 0 hsl(var(--primary));
  }
  .txt {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .label {
    font-weight: 500;
  }
  .sub {
    font-size: 11px;
  }
  .k {
    margin-left: auto;
    display: flex;
    gap: 4px;
    align-items: center;
  }
  .kbd {
    font-family: ui-monospace, "SF Mono", monospace;
    font-size: 10.5px;
    padding: 1px 5px;
    border-radius: 4px;
    background: hsl(var(--muted));
    border: 1px solid hsl(var(--border));
    color: hsl(var(--muted-foreground));
  }
  .none {
    padding: 16px;
    color: hsl(var(--muted-foreground));
    text-align: center;
  }
</style>
