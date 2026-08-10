// Pure keybinding parsing/formatting — no runes, no localStorage — so it stays
// unit-testable under plain `node`. 'mod' = Cmd on macOS, Ctrl everywhere else.

// Minimal shape of the KeyboardEvent fields we read (lets tests pass plain objects).
export type KeyLike = {
  key: string
  metaKey?: boolean
  shiftKey?: boolean
  altKey?: boolean
  ctrlKey?: boolean
}

export const isMac: boolean =
  typeof navigator !== 'undefined' && /Mac|iPhone|iPad|iPod/.test(navigator.platform)

// Normalize a key event to a binding string, or null for a bare modifier press.
// Modifiers always emit in a fixed order so equal chords compare equal.
// On macOS  Cmd  → 'mod'; on Windows/Linux  Ctrl → 'mod'.
export function eventToBinding(e: KeyLike, mac = isMac): string | null {
  const k = e.key.toLowerCase()
  if (k === 'meta' || k === 'shift' || k === 'alt' || k === 'control') return null
  const parts: string[] = []
  if (mac ? e.metaKey : e.ctrlKey) parts.push('mod')
  if (e.shiftKey) parts.push('shift')
  if (e.altKey) parts.push('alt')
  // Raw Ctrl is a distinct modifier only on macOS (e.g. ^A in terminal emulators).
  if (mac && e.ctrlKey) parts.push('ctrl')
  parts.push(k)
  return parts.join('+')
}

// True when the event would produce a character (a single key, at most Shift).
// Such chords must never be stolen from a focused terminal or text input — e.g.
// `?` must type in the shell, not open the shortcut sheet.
export function isPrintableChord(e: KeyLike): boolean {
  if (e.metaKey || e.ctrlKey || e.altKey) return false
  return e.key.length === 1
}

const MAC_SYMBOL: Record<string, string> = {
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

const WIN_SYMBOL: Record<string, string> = {
  mod: 'Ctrl',
  shift: 'Shift',
  alt: 'Alt',
  ctrl: 'Ctrl',
  arrowright: '→',
  arrowleft: '←',
  arrowup: '↑',
  arrowdown: '↓',
  ' ': 'Space',
}

// Pretty-print a binding for display.
// macOS:          'mod+shift+t' → '⌘⇧T'
// Windows/Linux:  'mod+shift+t' → 'Ctrl+Shift+T'
export function formatBinding(b: string, mac = isMac): string {
  const sym = mac ? MAC_SYMBOL : WIN_SYMBOL
  const sep = mac ? '' : '+'
  return b
    .split('+')
    .map((p) => sym[p] ?? (p.length === 1 ? p.toUpperCase() : p[0].toUpperCase() + p.slice(1)))
    .join(sep)
}
