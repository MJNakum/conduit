// Roving-tabindex list navigation (WAI-ARIA APG). Applied to a list container:
// exactly one item carries tabindex="0", the rest -1; Arrow keys move the "0",
// Home/End jump to the ends, Enter/Space activate. Items are matched by the
// `[data-roving-item]` attribute so callers keep their own markup.
//
// This centralizes what used to be per-component `sel` + window-keydown blocks.

export type RovingOptions = {
  // 'vertical' (Up/Down), 'horizontal' (Left/Right), or 'grid' (all four).
  orientation?: 'vertical' | 'horizontal' | 'grid'
  // Column count for grid orientation, so Up/Down jump a full row.
  columns?: number
  // Wrap past the ends instead of clamping.
  wrap?: boolean
  // Fired on Enter/Space with the focused item's index.
  onactivate?: (index: number, el: HTMLElement) => void
}

const SELECTOR = '[data-roving-item]:not([disabled]):not([hidden])'

export function roving(node: HTMLElement, options: RovingOptions = {}) {
  let opts = options

  const items = (): HTMLElement[] => Array.from(node.querySelectorAll<HTMLElement>(SELECTOR))

  // Set the single tab stop to `activeIdx`. Any `[data-roving-action]` buttons
  // inside an item (row Edit/Delete/etc.) join the Tab order only for the active
  // item, so the whole list is still one tab stop but the focused row's actions
  // are reachable with Tab.
  function apply(list: HTMLElement[], activeIdx: number) {
    list.forEach((el, i) => {
      const on = i === activeIdx
      el.tabIndex = on ? 0 : -1
      el.querySelectorAll<HTMLElement>('[data-roving-action]').forEach((a) => (a.tabIndex = on ? 0 : -1))
    })
  }

  // Keep tabindex in sync: the focused item (or the first) is the single tab stop.
  function sync() {
    const list = items()
    if (list.length === 0) return
    const active = document.activeElement as HTMLElement | null
    // If focus is inside an item (e.g. on a row action), keep that item active.
    const idx = list.findIndex((el) => el === active || el.contains(active))
    apply(list, idx === -1 ? 0 : idx)
  }

  function focusAt(list: HTMLElement[], i: number) {
    const el = list[i]
    if (!el) return
    apply(list, i)
    el.focus()
  }

  function move(list: HTMLElement[], from: number, delta: number) {
    let next = from + delta
    if (opts.wrap) next = (next + list.length) % list.length
    else next = Math.max(0, Math.min(list.length - 1, next))
    focusAt(list, next)
  }

  function onKeydown(e: KeyboardEvent) {
    const list = items()
    if (list.length === 0) return
    const current = list.indexOf(document.activeElement as HTMLElement)
    if (current === -1) return

    const orient = opts.orientation ?? 'vertical'
    const cols = opts.columns ?? 1
    const vertical = orient === 'vertical' || orient === 'grid'
    const horizontal = orient === 'horizontal' || orient === 'grid'
    const rowStep = orient === 'grid' ? cols : 1

    switch (e.key) {
      case 'ArrowDown':
        if (!vertical) return
        e.preventDefault()
        move(list, current, rowStep)
        break
      case 'ArrowUp':
        if (!vertical) return
        e.preventDefault()
        move(list, current, -rowStep)
        break
      case 'ArrowRight':
        if (!horizontal) return
        e.preventDefault()
        move(list, current, 1)
        break
      case 'ArrowLeft':
        if (!horizontal) return
        e.preventDefault()
        move(list, current, -1)
        break
      case 'Home':
        e.preventDefault()
        focusAt(list, 0)
        break
      case 'End':
        e.preventDefault()
        focusAt(list, list.length - 1)
        break
      case 'Enter':
      case ' ':
        // Only intercept activation when a handler is supplied. Native controls
        // (<button>, <a>) activate themselves on Enter/Space — stealing the key
        // here would suppress their click.
        if (opts.onactivate) {
          e.preventDefault()
          opts.onactivate(current, list[current])
        }
        break
    }
  }

  node.addEventListener('keydown', onKeydown)
  // Re-sync tabindex when focus enters/leaves so the tab stop tracks the user.
  node.addEventListener('focusin', sync)
  // Items can change (list filtered, tab opened); keep the tab stop valid.
  const mo = new MutationObserver(sync)
  mo.observe(node, { childList: true, subtree: true })
  sync()

  return {
    update(next: RovingOptions) {
      opts = next
    },
    destroy() {
      node.removeEventListener('keydown', onKeydown)
      node.removeEventListener('focusin', sync)
      mo.disconnect()
    },
  }
}
