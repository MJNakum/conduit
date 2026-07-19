import { invoke } from '@tauri-apps/api/core'
import { Database, Globe, Rocket, FlaskConical, Laptop, Server, type Icon } from '@lucide/svelte'

export type Host = {
  id: string
  name: string
  hostname: string
  port: number
  user: string
  tags: string[]
  color: string | null
  favorite: boolean
  group: string | null
  autoReconnect: boolean
  auth: 'password' | 'key'
  keyId: string | null // managed key from the Key Manager
  identityFile: string | null // or a raw private-key path
  jumps: string[] // ordered saved-host ids to ProxyJump through (bastion-1 … target)
  raw: string[] // verbatim ssh_config lines we don't model, kept for round-trip
}

export function blankHost(): Host {
  return {
    id: crypto.randomUUID(),
    name: '',
    hostname: '',
    port: 22,
    user: '',
    tags: [],
    color: null,
    favorite: false,
    group: null,
    autoReconnect: false,
    auth: 'password',
    keyId: null,
    identityFile: null,
    jumps: [],
    raw: [],
  }
}

// ---- ssh_config import / export (no lock-in) ------------------------------
export const importSshConfig = (path?: string) =>
  invoke<Host[]>('ssh_config_import', { path: path ?? null })
export const exportSshConfig = () => invoke<string>('ssh_config_export')
export const writeSshConfig = (path: string, text: string) =>
  invoke('ssh_config_export_write', { path, text })

// Resolve a host's jump chain (saved-host ids) into hop descriptors the backend
// can connect through; each jump's secret is pulled from the keychain by hostId.
export function resolveJumps(h: Host): unknown[] {
  return (h.jumps ?? [])
    .map((id) => store.hosts.find((x) => x.id === id))
    .filter((j): j is Host => !!j)
    .map((j) => ({
      hostId: j.id,
      host: j.hostname,
      port: j.port,
      user: j.user,
      auth: j.auth,
      keyId: j.keyId,
      identityFile: j.identityFile,
    }))
}

// ---- Managed SSH keys (Key Manager) ---------------------------------------
export type Key = {
  id: string
  name: string
  key_type: string // Algorithm name, e.g. "ssh-ed25519"
  fingerprint: string // "SHA256:…"
  public_key: string // authorized_keys line
  created: string // unix seconds
}

export const keysStore = $state({ keys: [] as Key[] })

export async function loadKeys() {
  keysStore.keys = await invoke<Key[]>('keys_list')
}

export async function generateKey(name: string, keyType: 'ed25519' | 'rsa' | 'ecdsa'): Promise<Key> {
  const k = await invoke<Key>('key_generate', { id: crypto.randomUUID(), name, keyType })
  keysStore.keys.push(k)
  return k
}

export async function importKey(name: string, pem: string, passphrase: string): Promise<Key> {
  const k = await invoke<Key>('key_import', {
    id: crypto.randomUUID(),
    name,
    pem,
    passphrase: passphrase || null,
  })
  keysStore.keys.push(k)
  return k
}

export async function deleteKey(id: string) {
  await invoke('key_delete', { id })
  keysStore.keys = keysStore.keys.filter((k) => k.id !== id)
}

// Auto-icon when the user hasn't picked one: a cheap keyword map, else a default.
// Returns a lucide icon component (no emoji anywhere — see CLAUDE.md).
export function hostIcon(h: Host): typeof Icon {
  const s = `${h.name} ${h.hostname}`.toLowerCase()
  if (/(^|\W)(db|postgres|mysql|redis|mongo)/.test(s)) return Database
  if (/(web|nginx|apache|www)/.test(s)) return Globe
  if (/(prod|production)/.test(s)) return Rocket
  if (/(dev|staging|test)/.test(s)) return FlaskConical
  if (/(local|localhost|127\.0\.0\.1)/.test(s)) return Laptop
  return Server
}

// Subsequence fuzzy match: returns a score (higher = better, consecutive-char
// bonus) or -1 if not every query char appears in order. No dep needed.
export function fuzzy(query: string, text: string): number {
  const q = query.toLowerCase()
  const t = text.toLowerCase()
  if (!q) return 0
  let qi = 0
  let score = 0
  let prev = -2
  for (let ti = 0; ti < t.length && qi < q.length; ti++) {
    if (t[ti] === q[qi]) {
      score += prev === ti - 1 ? 2 : 1
      prev = ti
      qi++
    }
  }
  return qi === q.length ? score : -1
}

// Single source of truth for saved hosts.
export const store = $state({ hosts: [] as Host[] })

export async function loadHosts() {
  store.hosts = await invoke<Host[]>('hosts_list')
}

export async function saveHost(h: Host) {
  const saved = await invoke<Host>('host_save', { host: h })
  const i = store.hosts.findIndex((x) => x.id === saved.id)
  if (i >= 0) store.hosts[i] = saved
  else store.hosts.push(saved)
}

export async function deleteHost(id: string) {
  await invoke('host_delete', { id })
  store.hosts = store.hosts.filter((h) => h.id !== id)
}

// ---- Tabs & panes ---------------------------------------------------------
// Tab 0 (the pinned "All Sessions" host list) is not a Tab object; it's the
// `active === 'home'` state. Each tab holds 1/2/4 panes; every pane is its own
// host + session, so a single tab can show several different hosts at once.

export type Pane = {
  key: string
  host: Host | null // null = empty pane awaiting a host pick
  sessionId: string | null
  phase: string // real ssh://state: '' | connecting | hostkey | authenticating | connected | disconnected | error
  error: string
  method: string // auth method for the current 'authenticating' phase
  // Host-key prompt payload (phase === 'hostkey'):
  keyHost: string // the machine being verified (a bastion mid-chain isn't the tab host)
  fingerprint: string
  keyType: string
  keyChanged: boolean
  oldFingerprint: string
}

export type Layout = 'single' | 'split2' | 'split4'
const PANE_COUNT: Record<Layout, number> = { single: 1, split2: 2, split4: 4 }

export type Tab = {
  key: string
  layout: Layout
  panes: Pane[]
  active: string // active pane key (focus target for split controls, broadcast later)
}

function newPane(host: Host | null): Pane {
  return {
    key: crypto.randomUUID(),
    host,
    sessionId: null,
    phase: '',
    error: '',
    method: '',
    keyHost: '',
    fingerprint: '',
    keyType: '',
    keyChanged: false,
    oldFingerprint: '',
  }
}

export const ui = $state({ tabs: [] as Tab[], active: 'home' as string })

export function openTab(host: Host): Tab {
  const pane = newPane(host)
  const tab: Tab = { key: crypto.randomUUID(), layout: 'single', panes: [pane], active: pane.key }
  ui.tabs.push(tab)
  ui.active = tab.key
  return tab
}

// Grow/shrink a tab's pane grid. Removed panes have their sessions torn down.
export function setLayout(tab: Tab, layout: Layout) {
  const want = PANE_COUNT[layout]
  while (tab.panes.length > want) {
    const p = tab.panes.pop()!
    if (p.sessionId) invoke('ssh_disconnect', { id: p.sessionId })
  }
  while (tab.panes.length < want) tab.panes.push(newPane(null))
  tab.layout = layout
  if (!tab.panes.some((p) => p.key === tab.active)) tab.active = tab.panes[0].key
}

export function closeTab(key: string) {
  const tab = ui.tabs.find((t) => t.key === key)
  tab?.panes.forEach((p) => p.sessionId && invoke('ssh_disconnect', { id: p.sessionId }))
  ui.tabs = ui.tabs.filter((t) => t.key !== key)
  if (ui.active === key) ui.active = 'home'
}

// The host that titles a tab: its first pane that has one.
export const tabHost = (tab: Tab): Host | null => tab.panes.find((p) => p.host)?.host ?? null

// The ssh://state event payload — a flattened ConnState from the Rust side.
export type StatePayload = {
  id: string
  state: string
  message?: string
  method?: string
  host?: string
  fingerprint?: string
  key_type?: string
  changed?: boolean
  old?: string | null
}

// Route a real connection-state event to its pane (matched by session id).
export function applyState(p: StatePayload) {
  for (const tab of ui.tabs) {
    const pane = tab.panes.find((x) => x.sessionId === p.id)
    if (!pane) continue
    pane.phase = p.state
    pane.error = p.state === 'error' ? (p.message ?? 'error') : ''
    if (p.state === 'authenticating') pane.method = p.method ?? ''
    if (p.state === 'hostkey') {
      pane.keyHost = p.host ?? ''
      pane.fingerprint = p.fingerprint ?? ''
      pane.keyType = p.key_type ?? ''
      pane.keyChanged = !!p.changed
      pane.oldFingerprint = p.old ?? ''
    }
    return
  }
}

export const activeCount = () =>
  ui.tabs.reduce((n, t) => n + t.panes.filter((p) => p.phase === 'connected').length, 0)
