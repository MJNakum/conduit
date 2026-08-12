<script lang="ts">
  // Themed dropdown. Native <select> popups are drawn by the OS and on Linux
  // (WebKitGTK) ignore our CSS entirely — rendering black/unreadable. This is our
  // own DOM so it looks identical and on-theme on every platform.
  import { ChevronDown, Check } from '@lucide/svelte'

  type Opt = { value: string; label: string }
  let {
    value = $bindable(''),
    options,
    onchange,
    id,
    disabled = false,
    placeholder = 'Select…',
    ariaLabel,
  }: {
    value?: string
    options: Opt[]
    onchange?: (v: string) => void
    id?: string
    disabled?: boolean
    placeholder?: string
    ariaLabel?: string
  } = $props()

  let open = $state(false)
  let active = $state(0)
  let root: HTMLDivElement
  let listEl = $state<HTMLDivElement>()

  const selected = $derived(options.find((o) => o.value === value))

  function toggle() {
    if (disabled) return
    open ? close() : openList()
  }
  function openList() {
    open = true
    active = Math.max(0, options.findIndex((o) => o.value === value))
    queueMicrotask(() => listEl?.querySelector<HTMLElement>('[data-active="true"]')?.scrollIntoView({ block: 'nearest' }))
  }
  function close() {
    open = false
  }
  function pick(o: Opt) {
    value = o.value
    onchange?.(o.value)
    close()
  }

  function onKey(e: KeyboardEvent) {
    if (disabled) return
    if (!open) {
      if (e.key === 'Enter' || e.key === ' ' || e.key === 'ArrowDown') {
        e.preventDefault()
        openList()
      }
      return
    }
    if (e.key === 'Escape') {
      e.preventDefault()
      close()
    } else if (e.key === 'ArrowDown') {
      e.preventDefault()
      active = Math.min(options.length - 1, active + 1)
      scrollActive()
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      active = Math.max(0, active - 1)
      scrollActive()
    } else if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault()
      if (options[active]) pick(options[active])
    } else if (e.key === 'Tab') {
      close()
    }
  }
  function scrollActive() {
    queueMicrotask(() => listEl?.querySelector<HTMLElement>('[data-active="true"]')?.scrollIntoView({ block: 'nearest' }))
  }

  // Close when focus/click leaves the component.
  function onDocPointer(e: PointerEvent) {
    if (open && root && !root.contains(e.target as Node)) close()
  }
  $effect(() => {
    if (!open) return
    document.addEventListener('pointerdown', onDocPointer, true)
    return () => document.removeEventListener('pointerdown', onDocPointer, true)
  })
</script>

<div class="sel" bind:this={root}>
  <button
    {id}
    type="button"
    class="trigger"
    class:open
    {disabled}
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-label={ariaLabel}
    onclick={toggle}
    onkeydown={onKey}
  >
    <span class="val" class:placeholder={!selected}>{selected ? selected.label : placeholder}</span>
    <ChevronDown size={15} class="chev" />
  </button>

  {#if open}
    <div class="list" role="listbox" bind:this={listEl} tabindex="-1">
      {#each options as o, i (o.value)}
        <div
          class="opt"
          role="option"
          aria-selected={o.value === value}
          data-active={i === active}
          onpointerenter={() => (active = i)}
          onclick={() => pick(o)}
        >
          <span class="tick">{#if o.value === value}<Check size={14} />{/if}</span>
          <span>{o.label}</span>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .sel {
    position: relative;
    display: block;
  }
  .trigger {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
    background: hsl(var(--muted));
    border: 1px solid hsl(var(--border));
    border-radius: 7px;
    padding: 8px 10px;
    color: hsl(var(--foreground));
    font: inherit;
    font-size: 13px;
    cursor: pointer;
    text-align: left;
  }
  .trigger:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .trigger.open,
  .trigger:focus-visible {
    border-color: hsl(var(--ring) / 0.6);
  }
  .val {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .val.placeholder {
    color: hsl(var(--muted-foreground));
  }
  .trigger :global(.chev) {
    flex: none;
    color: hsl(var(--muted-foreground));
    transition: transform var(--dur-fast) var(--ease);
  }
  .trigger.open :global(.chev) {
    transform: rotate(180deg);
  }
  .list {
    position: absolute;
    z-index: 90;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    max-height: 260px;
    overflow-y: auto;
    background: hsl(var(--popover));
    border: 1px solid hsl(var(--border));
    border-radius: 8px;
    padding: 4px;
    box-shadow: 0 8px 24px hsl(0 0% 0% / 0.35);
  }
  .opt {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 8px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 13px;
    color: hsl(var(--foreground));
  }
  .opt[data-active='true'] {
    background: hsl(var(--muted));
  }
  .opt[aria-selected='true'] {
    color: hsl(var(--primary));
    font-weight: 600;
  }
  .tick {
    display: inline-flex;
    width: 14px;
    flex: none;
    color: hsl(var(--primary));
  }
</style>
