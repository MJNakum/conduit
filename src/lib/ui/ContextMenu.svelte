<script lang="ts">
  // Right-click menu, positioned at the pointer. Fixed rather than absolute so
  // it escapes the tab bar's overflow, and clamped to the viewport so a tab near
  // the right edge doesn't push it off-screen. Dismiss mirrors Select.svelte: a
  // capture-phase pointerdown outside, plus Escape.
  import type { Icon } from '@lucide/svelte'

  export type MenuItem = {
    label: string
    icon?: typeof Icon
    danger?: boolean
    disabled?: boolean
    onselect: () => void
  }

  let {
    x,
    y,
    items,
    onclose,
    ariaLabel = 'Context menu',
  }: {
    x: number
    y: number
    items: MenuItem[]
    onclose: () => void
    ariaLabel?: string
  } = $props()

  let el = $state<HTMLDivElement>()
  let size = $state({ w: 0, h: 0 })
  let active = $state(0)

  const enabled = $derived(items.filter((i) => !i.disabled))

  // Measure after mount; until then w/h are 0 and the menu simply sits at the
  // pointer, which is already right in the common case.
  $effect(() => {
    if (!el) return
    const r = el.getBoundingClientRect()
    size = { w: r.width, h: r.height }
  })

  // Derived from the props rather than assigned once, so reopening the menu on
  // a different tab repositions it even when Svelte reuses this instance.
  // Flipping rather than merely clamping keeps the pointer outside the menu, so
  // the press that opened it can't land on an item.
  const pos = $derived.by(() => {
    const pad = 8
    return {
      x: x + size.w + pad > window.innerWidth ? Math.max(pad, x - size.w) : x,
      y: y + size.h + pad > window.innerHeight ? Math.max(pad, y - size.h) : y,
    }
  })

  // Focus once so arrow keys and Escape work without a click.
  $effect(() => {
    el?.focus()
  })

  function pick(item: MenuItem) {
    if (item.disabled) return
    onclose()
    item.onselect()
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault()
      e.stopPropagation()
      onclose()
    } else if (e.key === 'ArrowDown') {
      e.preventDefault()
      active = (active + 1) % Math.max(1, enabled.length)
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      active = (active - 1 + Math.max(1, enabled.length)) % Math.max(1, enabled.length)
    } else if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault()
      if (enabled[active]) pick(enabled[active])
    } else if (e.key === 'Tab') {
      onclose()
    }
  }

  // Any pointer press outside closes. Capture phase so it wins over whatever is
  // underneath, and the press still reaches its target afterwards.
  function onDocPointer(e: PointerEvent) {
    if (el && !el.contains(e.target as Node)) onclose()
  }
  $effect(() => {
    document.addEventListener('pointerdown', onDocPointer, true)
    // Scrolling or resizing leaves the menu stranded away from what it points at.
    window.addEventListener('resize', onclose)
    window.addEventListener('blur', onclose)
    document.addEventListener('scroll', onclose, true)
    return () => {
      document.removeEventListener('pointerdown', onDocPointer, true)
      window.removeEventListener('resize', onclose)
      window.removeEventListener('blur', onclose)
      document.removeEventListener('scroll', onclose, true)
    }
  })
</script>

<div
  class="menu"
  role="menu"
  aria-label={ariaLabel}
  tabindex="-1"
  bind:this={el}
  style:left="{pos.x}px"
  style:top="{pos.y}px"
  onkeydown={onKey}
>
  {#each items as item (item.label)}
    {@const i = enabled.indexOf(item)}
    <button
      class="item"
      class:danger={item.danger}
      role="menuitem"
      disabled={item.disabled}
      data-active={i >= 0 && i === active}
      onpointerenter={() => { if (i >= 0) active = i }}
      onclick={() => pick(item)}
    >
      <span class="ico">{#if item.icon}<item.icon size={14} />{/if}</span>
      <span>{item.label}</span>
    </button>
  {/each}
</div>

<style>
  .menu {
    position: fixed;
    z-index: 95;
    min-width: 200px;
    padding: 5px;
    border: 1px solid hsl(var(--border));
    border-radius: 9px;
    background: hsl(var(--popover));
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.45);
    outline: none;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    padding: 7px 10px;
    border: none;
    border-radius: 6px;
    background: none;
    color: hsl(var(--foreground));
    font: inherit;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }
  .item[data-active='true'] {
    background: hsl(var(--muted));
  }
  .item.danger {
    color: hsl(var(--destructive));
  }
  .item:disabled {
    opacity: 0.5;
    cursor: default;
    background: none;
  }
  .ico {
    display: inline-flex;
    width: 14px;
    flex: none;
    color: hsl(var(--muted-foreground));
  }
</style>
