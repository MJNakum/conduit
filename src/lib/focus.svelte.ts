// Region focus model — the "Focus Next Part" (F6) navigation VS Code popularized.
// Regions register their container element; cycleRegion() moves focus to the
// next/previous region's primary tab stop. The active region is tracked from
// focusin so F6 always advances from wherever the user currently is.

export type RegionName = 'tabbar' | 'sidebar' | 'content' | 'footer'

// Cycle order matches the on-screen reading order (tab bar sits above the body).
const ORDER: RegionName[] = ['tabbar', 'sidebar', 'content', 'footer']

const regions = new Map<RegionName, HTMLElement>()

export const focusState = $state<{ active: RegionName }>({ active: 'content' })

export function registerRegion(name: RegionName, el: HTMLElement) {
  regions.set(name, el)
  return () => {
    if (regions.get(name) === el) regions.delete(name)
  }
}

// Svelte action: `use:region={'sidebar'}` registers the element as a region.
export function region(node: HTMLElement, name: RegionName) {
  let cleanup = registerRegion(name, node)
  return {
    update(next: RegionName) {
      cleanup()
      cleanup = registerRegion(next, node)
    },
    destroy: () => cleanup(),
  }
}

// Note the region a focus event landed in, so cycling advances from here.
export function noteFocus(target: EventTarget | null) {
  if (!(target instanceof Node)) return
  for (const [name, el] of regions) {
    if (el.contains(target)) {
      focusState.active = name
      return
    }
  }
}

// Focus a region's primary tab stop: the current roving item (tabindex 0),
// else the first focusable descendant, else the container itself.
export function focusRegion(name: RegionName): boolean {
  const el = regions.get(name)
  if (!el) return false
  const target =
    el.querySelector<HTMLElement>('[data-roving-item][tabindex="0"]') ??
    el.querySelector<HTMLElement>('[data-roving-item]') ??
    el.querySelector<HTMLElement>(
      'a[href],button:not([disabled]),input:not([disabled]),select:not([disabled]),textarea:not([disabled]),[tabindex]:not([tabindex="-1"])',
    ) ??
    el
  if (target === el && !el.hasAttribute('tabindex')) el.tabIndex = -1
  target.focus()
  focusState.active = name
  return true
}

// Move focus to the next (dir=1) or previous (dir=-1) registered region,
// skipping any that aren't currently mounted.
export function cycleRegion(dir: 1 | -1) {
  const present = ORDER.filter((n) => regions.has(n))
  if (present.length === 0) return
  const from = present.indexOf(focusState.active)
  const start = from === -1 ? 0 : from
  const n = present.length
  // Try each subsequent region until one accepts focus.
  for (let step = 1; step <= n; step++) {
    const next = present[(((start + dir * step) % n) + n) % n]
    if (focusRegion(next)) return
  }
}
