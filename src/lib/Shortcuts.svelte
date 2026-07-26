<script lang="ts">
  import { Keyboard, RotateCcw } from '@lucide/svelte'
  import {
    ACTIONS,
    bindingOf,
    setBinding,
    resetBinding,
    resetAll,
    conflictOf,
    eventToBinding,
    formatBinding,
    type ActionId,
    type Action,
  } from './keymap.svelte'
  import { toast } from './toast.svelte'

  let recording = $state<ActionId | null>(null)
  let conflict = $state('')

  // Group actions by category, preserving declaration order.
  const groups = $derived.by(() => {
    const out: { name: string; items: Action[] }[] = []
    for (const a of ACTIONS) {
      let g = out.find((x) => x.name === a.category)
      if (!g) out.push((g = { name: a.category, items: [] }))
      g.items.push(a)
    }
    return out
  })

  function record(id: ActionId) {
    recording = recording === id ? null : id
    conflict = ''
  }

  // While recording, capture the next chord globally (capture phase +
  // stopImmediatePropagation) so no app shortcut fires during rebinding.
  $effect(() => {
    if (!recording) return
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault()
      e.stopImmediatePropagation()
      if (e.key === 'Escape') {
        recording = null
        return
      }
      const b = eventToBinding(e)
      if (!b) return // bare modifier — keep waiting
      const c = conflictOf(recording!, b)
      if (c) {
        conflict = `${formatBinding(b)} is already "${c.label}"`
        return
      }
      setBinding(recording!, b)
      toast(`Shortcut set to ${formatBinding(b)}`)
      recording = null
    }
    window.addEventListener('keydown', onKey, true)
    return () => window.removeEventListener('keydown', onKey, true)
  })

  function reset(id: ActionId) {
    resetBinding(id)
    toast('Shortcut reset to default')
  }
  function resetEverything() {
    resetAll()
    toast('All shortcuts reset')
  }
</script>

<div class="sc">
  <div class="hd">
    <span class="title"><Keyboard size={15} /> Keyboard shortcuts</span>
    <button class="btn" onclick={resetEverything}><RotateCcw size={13} /> Reset all</button>
  </div>
  <p class="hint muted">Click a shortcut, then press the new key combination. Esc cancels.</p>

  {#each groups as g (g.name)}
    <div class="cat">{g.name}</div>
    {#each g.items as a (a.id)}
      {@const b = bindingOf(a.id)}
      {@const custom = b !== a.def}
      <div class="row">
        <span class="label">{a.label}</span>
        <span class="spacer"></span>
        {#if custom}
          <button class="reset" title="Reset to default" aria-label="Reset" onclick={() => reset(a.id)}>
            <RotateCcw size={12} />
          </button>
        {/if}
        <button class="chip" class:rec={recording === a.id} class:custom onclick={() => record(a.id)}>
          {recording === a.id ? 'Press keys…' : formatBinding(b)}
        </button>
      </div>
    {/each}
  {/each}

  {#if conflict}<div class="err">{conflict}</div>{/if}
</div>

<style>
  .sc {
    max-width: 640px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .hd {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 2px;
  }
  .title {
    display: flex;
    align-items: center;
    gap: 9px;
    font-size: 14px;
    font-weight: 600;
  }
  .hint {
    font-size: 12px;
    margin: 0 0 8px;
  }
  .spacer {
    flex: 1;
  }
  .cat {
    padding: 12px 2px 4px;
    font-size: 10.5px;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: hsl(var(--muted-foreground));
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 4px;
    border-bottom: 1px solid hsl(var(--border) / 0.5);
  }
  .label {
    font-size: 13px;
  }
  .chip {
    min-width: 74px;
    padding: 5px 11px;
    border: 1px solid hsl(var(--border));
    border-radius: 7px;
    background: hsl(var(--muted));
    color: hsl(var(--foreground));
    font-family: ui-monospace, "SF Mono", monospace;
    font-size: 12px;
    cursor: pointer;
    text-align: center;
  }
  .chip:hover {
    border-color: hsl(var(--ring) / 0.6);
  }
  .chip.custom {
    border-color: hsl(var(--primary) / 0.5);
  }
  .chip.rec {
    border-color: hsl(var(--connecting));
    color: hsl(var(--connecting));
    background: hsl(var(--connecting) / 0.12);
  }
  .reset {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border: none;
    background: none;
    border-radius: 6px;
    color: hsl(var(--muted-foreground));
    cursor: pointer;
  }
  .reset:hover {
    background: hsl(var(--muted));
    color: hsl(var(--foreground));
  }
  .btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 11px;
    border: 1px solid hsl(var(--border));
    border-radius: 7px;
    background: hsl(var(--muted));
    color: inherit;
    font-size: 12px;
    font-family: inherit;
    cursor: pointer;
  }
  .btn:hover {
    background: hsl(var(--border));
  }
  .err {
    margin-top: 8px;
    color: hsl(var(--destructive));
    font-size: 12.5px;
  }
</style>
