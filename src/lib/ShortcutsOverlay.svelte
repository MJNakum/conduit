<script lang="ts">
  import { ACTIONS, bindingOf, formatBinding } from './keymap.svelte'
  import { trapFocus } from './actions/trapFocus'

  let { onclose }: { onclose: () => void } = $props()

  // Group actions by category, preserving the registry's display order.
  const groups = $derived.by(() => {
    const order: string[] = []
    const by = new Map<string, typeof ACTIONS>()
    for (const a of ACTIONS) {
      if (!by.has(a.category)) {
        by.set(a.category, [])
        order.push(a.category)
      }
      by.get(a.category)!.push(a)
    }
    return order.map((category) => ({ category, items: by.get(category)! }))
  })
</script>

<div class="backdrop" onclick={onclose} role="presentation">
  <div
    class="sheet"
    onclick={(e) => e.stopPropagation()}
    role="dialog"
    tabindex="-1"
    aria-label="Keyboard shortcuts"
    aria-modal="true"
    use:trapFocus={{ onclose }}
  >
    <div class="head">
      <h2>Keyboard shortcuts</h2>
      <span class="kbd">esc</span>
    </div>
    <div class="cols">
      {#each groups as g (g.category)}
        <div class="group">
          <div class="gtitle">{g.category}</div>
          {#each g.items as a (a.id)}
            <div class="krow">
              <span class="label">{a.label}</span>
              <span class="kbd mono">{formatBinding(bindingOf(a.id))}</span>
            </div>
          {/each}
        </div>
      {/each}
    </div>
    <div class="foot muted">Rebind any of these in Settings &rsaquo; Shortcuts.</div>
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
    padding-top: 10vh;
    z-index: 70;
  }
  .sheet {
    width: 720px;
    max-width: 92vw;
    max-height: 78vh;
    display: flex;
    flex-direction: column;
    background: hsl(var(--popover));
    border: 1px solid hsl(var(--border));
    border-radius: 12px;
    box-shadow: 0 24px 64px rgba(0, 0, 0, 0.55);
    overflow: hidden;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 18px;
    border-bottom: 1px solid hsl(var(--border));
  }
  .head h2 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
  }
  .cols {
    padding: 8px 18px 4px;
    overflow-y: auto;
    columns: 2;
    column-gap: 28px;
  }
  .group {
    break-inside: avoid;
    margin-bottom: 16px;
  }
  .gtitle {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: hsl(var(--muted-foreground));
    padding: 4px 0 6px;
  }
  .krow {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 4px 0;
    font-size: 12.5px;
  }
  .label {
    color: hsl(var(--foreground));
  }
  .kbd {
    font-size: 10.5px;
    padding: 2px 6px;
    border-radius: 4px;
    background: hsl(var(--muted));
    border: 1px solid hsl(var(--border));
    color: hsl(var(--muted-foreground));
    white-space: nowrap;
  }
  .foot {
    padding: 10px 18px;
    border-top: 1px solid hsl(var(--border));
    font-size: 11.5px;
  }
</style>
