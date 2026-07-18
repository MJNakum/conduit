<script lang="ts">
  import HostModal from './HostModal.svelte'
  import { store, deleteHost, hostIcon, blankHost, type Host } from './state.svelte'

  let { onopen }: { onopen: (h: Host) => void } = $props()

  let filter = $state('')
  let editing = $state<Host | null>(null)

  const filtered = $derived(
    store.hosts.filter((h) => {
      const q = filter.toLowerCase().trim()
      if (!q) return true
      return (
        h.name.toLowerCase().includes(q) ||
        h.hostname.toLowerCase().includes(q) ||
        h.tags.some((t) => t.toLowerCase().includes(q))
      )
    }),
  )
</script>

<header>
  <h1>Hosts</h1>
  <input class="filter" placeholder="filter by name or tag" bind:value={filter} />
  <button class="primary" onclick={() => (editing = blankHost())}>+ New host</button>
</header>

<ul class="hosts">
  {#each filtered as h (h.id)}
    {@const Icon = hostIcon(h)}
    <li>
      <button class="open" onclick={() => onopen(h)}>
        <span class="icon" style:color={h.color ?? undefined}><Icon size={18} /></span>
        <span class="meta">
          <span class="name">{h.name} {#if h.favorite}★{/if}</span>
          <span class="dim">{h.user}@{h.hostname}:{h.port}</span>
        </span>
        <span class="tags">{#each h.tags as t}<span class="tag">{t}</span>{/each}</span>
      </button>
      <span class="rowactions">
        <button onclick={() => (editing = { ...h })}>Edit</button>
        <button onclick={() => deleteHost(h.id)}>Delete</button>
      </span>
    </li>
  {:else}
    <li class="empty">No hosts yet. Add one to get started.</li>
  {/each}
</ul>

{#if editing}
  <HostModal host={editing} onclose={() => (editing = null)} />
{/if}

<style>
  header {
    display: flex;
    align-items: center;
    gap: 0.8rem;
    padding: 0.8rem 1rem;
    border-bottom: 1px solid #222;
  }
  header h1 {
    font-size: 1.1rem;
    margin: 0;
  }
  .filter {
    flex: 1;
    padding: 0.45rem;
    background: #1a1a1a;
    border: 1px solid #333;
    color: #eee;
    border-radius: 4px;
  }
  .hosts {
    list-style: none;
    margin: 0;
    padding: 0.5rem;
    overflow-y: auto;
  }
  .hosts li {
    display: flex;
    align-items: center;
    border-radius: 6px;
  }
  .hosts li:hover {
    background: #1a1a1a;
  }
  .open {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 0.8rem;
    padding: 0.6rem;
    background: none;
    border: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }
  .icon {
    display: flex;
    color: #9aa;
    flex: none;
  }
  .meta {
    display: flex;
    flex-direction: column;
  }
  .name {
    font-weight: 600;
  }
  .dim {
    color: #888;
    font-size: 0.8rem;
  }
  .tags {
    margin-left: auto;
    display: flex;
    gap: 0.3rem;
  }
  .tag {
    background: #263042;
    color: #9cc;
    padding: 0.1rem 0.4rem;
    border-radius: 999px;
    font-size: 0.7rem;
  }
  .rowactions {
    display: flex;
    gap: 0.3rem;
    padding-right: 0.5rem;
  }
  .rowactions button {
    font-size: 0.75rem;
    padding: 0.3rem 0.5rem;
    background: #222;
    border: 1px solid #333;
    color: #ccc;
    border-radius: 4px;
    cursor: pointer;
  }
  .empty {
    color: #777;
    padding: 1rem;
    justify-content: center;
  }
  button.primary {
    background: #2b6cff;
    border: 1px solid #2b6cff;
    color: #fff;
    padding: 0.45rem 0.9rem;
    border-radius: 4px;
    cursor: pointer;
  }
</style>
