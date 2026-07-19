<script lang="ts">
  import { Radio, X } from '@lucide/svelte'
  import { broadcast, connectedSessions, broadcastTargets, broadcastLine } from './state.svelte'

  let text = $state('')
  const targets = $derived(broadcastTargets())

  function send() {
    if (!text.trim() || targets.length === 0) return
    broadcastLine(text)
    text = ''
  }

  function onkey(e: KeyboardEvent) {
    if (e.key === 'Enter') { e.preventDefault(); send() }
    else if (e.key === 'Escape') { broadcast.on = false }
  }

  function exclude(id: string) {
    broadcast.exclude = [...broadcast.exclude, id]
  }
</script>

<div class="bar">
  <span class="lead"><Radio size={15} /> Broadcasting to {targets.length} session{targets.length === 1 ? '' : 's'}</span>
  <div class="chips">
    {#each connectedSessions() as s (s.id)}
      {@const on = !broadcast.exclude.includes(s.id)}
      <span class="chip" class:off={!on}>
        {s.name}
        {#if on}<button class="cx" aria-label="Exclude" onclick={() => exclude(s.id)}><X size={11} /></button>{/if}
      </span>
    {/each}
  </div>
  <!-- svelte-ignore a11y_autofocus -->
  <input bind:value={text} onkeydown={onkey} placeholder="Type a command, Enter to send to all…" autofocus />
  <label class="once"><input type="checkbox" bind:checked={broadcast.once} /> one-shot</label>
  <button class="off-btn" onclick={() => (broadcast.on = false)}>Esc to exit</button>
</div>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 14px;
    background: hsl(var(--amber) / 0.14);
    border-top: 2px solid hsl(var(--amber));
    color: hsl(var(--foreground));
    font-size: 12.5px;
  }
  .lead {
    display: flex;
    align-items: center;
    gap: 7px;
    font-weight: 600;
    color: hsl(var(--amber));
    flex: none;
  }
  .chips {
    display: flex;
    gap: 5px;
    flex-wrap: wrap;
    flex: none;
    max-width: 34%;
    overflow: hidden;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 7px;
    border-radius: 999px;
    background: hsl(var(--amber) / 0.2);
    border: 1px solid hsl(var(--amber) / 0.5);
    font-size: 11px;
    white-space: nowrap;
  }
  .chip.off {
    opacity: 0.4;
    text-decoration: line-through;
    background: hsl(var(--muted));
    border-color: hsl(var(--border));
  }
  .cx {
    display: grid;
    place-items: center;
    border: none;
    background: none;
    color: inherit;
    cursor: pointer;
    padding: 0;
  }
  input {
    flex: 1;
    background: hsl(var(--background));
    border: 1px solid hsl(var(--amber) / 0.5);
    border-radius: 7px;
    padding: 7px 10px;
    color: inherit;
    outline: none;
    font-family: inherit;
    font-size: 13px;
  }
  input:focus {
    border-color: hsl(var(--amber));
  }
  .once {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: none;
    color: hsl(var(--muted-foreground));
  }
  .once input {
    width: auto;
    flex: none;
  }
  .off-btn {
    flex: none;
    background: none;
    border: 1px solid hsl(var(--border));
    border-radius: 6px;
    color: hsl(var(--muted-foreground));
    padding: 6px 10px;
    cursor: pointer;
    font: inherit;
    font-size: 12px;
  }
  .off-btn:hover {
    background: hsl(var(--muted));
  }
</style>
