// Central, customizable keyboard shortcuts. Actions are named and carry a
// default binding; user overrides persist in localStorage. App owns the actual
// handlers (they need app state) and dispatches via matchEvent(); the Settings
// panel edits bindings via set/reset. 'mod' = Cmd on macOS.
import { eventToBinding, formatBinding } from './keys'
export { eventToBinding, formatBinding, isPrintableChord } from './keys'

export type ActionId =
  | 'palette'
  | 'settings'
  | 'cycleTheme'
  | 'newTab'
  | 'closeTab'
  | 'nextTab'
  | 'prevTab'
  | 'lockVault'
  | 'broadcast'
  | 'gotoHosts'
  | 'gotoKeys'
  | 'gotoSnippets'
  | 'gotoForwards'
  | 'gotoSettings'
  | 'gotoHistory'
  | 'cycleRegionNext'
  | 'cycleRegionPrev'
  | 'focusTerminal'
  | 'help'

export type Action = { id: ActionId; label: string; category: string; def: string }

// The full set, in display order. `def` is the default binding string.
export const ACTIONS: Action[] = [
  { id: 'palette', label: 'Command palette', category: 'General', def: 'mod+k' },
  { id: 'settings', label: 'Open settings', category: 'General', def: 'mod+,' },
  { id: 'cycleTheme', label: 'Cycle theme (dark / light / system)', category: 'General', def: 'mod+shift+t' },
  { id: 'lockVault', label: 'Lock vault', category: 'General', def: 'mod+l' },
  { id: 'newTab', label: 'New tab (Sessions)', category: 'Tabs', def: 'mod+t' },
  { id: 'closeTab', label: 'Close current tab', category: 'Tabs', def: 'mod+w' },
  { id: 'nextTab', label: 'Next tab', category: 'Tabs', def: 'mod+shift+arrowright' },
  { id: 'prevTab', label: 'Previous tab', category: 'Tabs', def: 'mod+shift+arrowleft' },
  { id: 'broadcast', label: 'Toggle broadcast', category: 'Session', def: 'mod+b' },
  { id: 'gotoHosts', label: 'Go to Hosts', category: 'Navigation', def: 'mod+1' },
  { id: 'gotoKeys', label: 'Go to Keys', category: 'Navigation', def: 'mod+2' },
  { id: 'gotoSnippets', label: 'Go to Snippets', category: 'Navigation', def: 'mod+3' },
  { id: 'gotoForwards', label: 'Go to Port Forwards', category: 'Navigation', def: 'mod+4' },
  { id: 'gotoSettings', label: 'Go to Settings', category: 'Navigation', def: 'mod+5' },
  { id: 'gotoHistory', label: 'Go to History', category: 'Navigation', def: 'mod+6' },
  { id: 'cycleRegionNext', label: 'Focus next region', category: 'Navigation', def: 'f6' },
  { id: 'cycleRegionPrev', label: 'Focus previous region', category: 'Navigation', def: 'shift+f6' },
  { id: 'focusTerminal', label: 'Focus the terminal', category: 'Navigation', def: 'mod+j' },
  { id: 'help', label: 'Keyboard shortcuts', category: 'General', def: 'shift+/' },
]

const LS_KEY = 'ssh.keymap'

// Reactive override map: ActionId -> binding string. Empty = use defaults.
export const keymap = $state<{ overrides: Record<string, string> }>({
  overrides: JSON.parse(localStorage.getItem(LS_KEY) ?? '{}'),
})

function persist() {
  localStorage.setItem(LS_KEY, JSON.stringify(keymap.overrides))
}

export function bindingOf(id: ActionId): string {
  return keymap.overrides[id] ?? ACTIONS.find((a) => a.id === id)!.def
}

export function setBinding(id: ActionId, b: string) {
  keymap.overrides[id] = b
  persist()
}

export function resetBinding(id: ActionId) {
  delete keymap.overrides[id]
  persist()
}

export function resetAll() {
  keymap.overrides = {}
  persist()
}

// The action currently bound to `b`, if any (for conflict detection).
export function conflictOf(id: ActionId, b: string): Action | null {
  return ACTIONS.find((a) => a.id !== id && bindingOf(a.id) === b) ?? null
}

// Match a keydown to an action id, or null.
export function matchEvent(e: KeyboardEvent): ActionId | null {
  const b = eventToBinding(e)
  if (!b) return null
  return ACTIONS.find((a) => bindingOf(a.id) === b)?.id ?? null
}
