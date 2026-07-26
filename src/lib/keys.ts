// Pure keybinding parsing/formatting — no runes, no localStorage — so it stays
// unit-testable under plain `node`. 'mod' = Cmd on macOS.

// Minimal shape of the KeyboardEvent fields we read (lets tests pass plain objects).
export type KeyLike = {
  key: string
  metaKey?: boolean
  shiftKey?: boolean
  altKey?: boolean
  ctrlKey?: boolean
}

// Normalize a key event to a binding string, or null for a bare modifier.
// Modifiers always emit in a fixed order so equal chords compare equal.
export function eventToBinding(e: KeyLike): string | null {
  const k = e.key.toLowerCase()
  if (k === 'meta' || k === 'shift' || k === 'alt' || k === 'control') return null
  const parts: string[] = []
  if (e.metaKey) parts.push('mod')
  if (e.shiftKey) parts.push('shift')
  if (e.altKey) parts.push('alt')
  if (e.ctrlKey) parts.push('ctrl')
  parts.push(k)
  return parts.join('+')
}

const SYMBOL: Record<string, string> = {
  mod: '⌘',
  shift: '⇧',
  alt: '⌥',
  ctrl: '⌃',
  arrowright: '→',
  arrowleft: '←',
  arrowup: '↑',
  arrowdown: '↓',
  ' ': 'Space',
}

// Pretty-print a binding for display: 'mod+shift+t' -> '⌘⇧T'.
export function formatBinding(b: string): string {
  return b
    .split('+')
    .map((p) => SYMBOL[p] ?? (p.length === 1 ? p.toUpperCase() : p[0].toUpperCase() + p.slice(1)))
    .join('')
}
