// Modal focus management (WAI-ARIA APG dialog pattern). Applied to a dialog
// panel: on mount records the previously-focused element and moves focus inside;
// Tab/Shift+Tab cycle within the panel; Escape calls onclose; on destroy focus
// returns to wherever it was. Replaces the inert `tabindex="-1"` panels that
// trapped nothing and ignored Escape.

export type TrapOptions = {
  // Called on Escape (and available for the caller's own close button).
  onclose?: () => void
}

const FOCUSABLE = [
  'a[href]',
  'button:not([disabled])',
  'input:not([disabled])',
  'select:not([disabled])',
  'textarea:not([disabled])',
  '[tabindex]:not([tabindex="-1"])',
].join(',')

export function trapFocus(node: HTMLElement, options: TrapOptions = {}) {
  let opts = options
  const previouslyFocused = document.activeElement as HTMLElement | null

  const focusable = (): HTMLElement[] =>
    Array.from(node.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(
      (el) => el.offsetParent !== null || el === document.activeElement,
    )

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault()
      e.stopPropagation()
      opts.onclose?.()
      return
    }
    if (e.key !== 'Tab') return
    const items = focusable()
    if (items.length === 0) {
      // Nothing focusable inside — keep focus on the panel itself.
      e.preventDefault()
      node.focus()
      return
    }
    const first = items[0]
    const last = items[items.length - 1]
    const active = document.activeElement
    if (e.shiftKey && (active === first || !node.contains(active))) {
      e.preventDefault()
      last.focus()
    } else if (!e.shiftKey && active === last) {
      e.preventDefault()
      first.focus()
    }
  }

  node.addEventListener('keydown', onKeydown)

  // Move focus into the panel: first focusable, else the panel itself.
  const initial = focusable()[0] ?? node
  // Panel needs to be focusable as a fallback target.
  if (initial === node && !node.hasAttribute('tabindex')) node.tabIndex = -1
  // Defer so the element is laid out (offsetParent) before we focus.
  queueMicrotask(() => initial.focus())

  return {
    update(next: TrapOptions) {
      opts = next
    },
    destroy() {
      node.removeEventListener('keydown', onKeydown)
      previouslyFocused?.focus?.()
    },
  }
}
