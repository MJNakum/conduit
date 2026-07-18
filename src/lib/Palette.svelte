<script lang="ts">
  import { tick } from 'svelte'
  import { X, type Icon } from '@lucide/svelte'
  import { store, ui, openTab, closeTab, hostIcon, tabHost, fuzzy } from './state.svelte'

  let { onclose }: { onclose: () => void } = $props()

  let query = $state('')
  let sel = $state(0)
  let input = $state<HTMLInputElement>()

  type Item = { label: string; sub: string; icon: typeof Icon; run: () => void }

  // Hosts (open in a new tab) + a close-tab action when a terminal tab is active.
  const items = $derived.by<Item[]>(() => {
    const list: Item[] = store.hosts.map((h) => ({
      label: h.name,
      sub: `${h.user}@${h.hostname}:${h.port}`,
      icon: hostIcon(h),
      run: () => {
        openTab(h)
        onclose()
      },
    }))
    if (ui.active !== 'home') {
      const key = ui.active
      list.push({
        label: 'Close current tab',
        sub: tabHost(ui.tabs.find((t) => t.key === key)!)?.name ?? '',
        icon: X,
        run: () => {
          closeTab(key)
          onclose()
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

  // Keep the selection in range as the list shrinks.
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
  <div class="palette" onclick={(e) => e.stopPropagation()} role="dialog" tabindex="-1">
    <input
      bind:this={input}
      bind:value={query}
      onkeydown={onKey}
      placeholder="Search hosts and actions…"
    />
    <ul>
      {#each filtered as item, i (item.label + item.sub)}
        {@const ItemIcon = item.icon}
        <li>
          <button class:active={i === sel} onclick={item.run} onmouseenter={() => (sel = i)}>
            <span class="ic"><ItemIcon size={16} /></span>
            <span class="label">{item.label}</span>
            <span class="sub">{item.sub}</span>
          </button>
        </li>
      {:else}
        <li class="empty">No matches</li>
      {/each}
    </ul>
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
  }
  .palette {
    width: 520px;
    max-width: 90vw;
    background: #1a1a1a;
    border: 1px solid #333;
    border-radius: 10px;
    overflow: hidden;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.5);
  }
  input {
    width: 100%;
    padding: 0.8rem 1rem;
    background: #111;
    border: none;
    border-bottom: 1px solid #2a2a2a;
    color: #eee;
    font-size: 1rem;
  }
  input:focus {
    outline: none;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0.3rem;
    max-height: 50vh;
    overflow-y: auto;
  }
  button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 0.7rem;
    padding: 0.55rem 0.7rem;
    background: none;
    border: none;
    color: #ddd;
    text-align: left;
    border-radius: 6px;
    cursor: pointer;
  }
  button.active {
    background: #2b6cff;
    color: #fff;
  }
  .ic {
    display: flex;
    color: inherit;
  }
  .label {
    font-weight: 600;
  }
  .sub {
    margin-left: auto;
    color: #999;
    font-size: 0.8rem;
  }
  button.active .sub {
    color: #cfe0ff;
  }
  .empty {
    padding: 0.8rem;
    color: #777;
    text-align: center;
  }
</style>
