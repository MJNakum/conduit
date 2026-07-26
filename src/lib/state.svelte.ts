import { invoke } from '@tauri-apps/api/core'
import { Database, Globe, Rocket, FlaskConical, Laptop, Server, type Icon } from '@lucide/svelte'
import { phaseStep, type Step, type LogLine } from './connsteps'

export type Host = {
  id: string
  name: string
  hostname: string
  port: number
  user: string
  protocol: 'ssh' | 'telnet'
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
  scheme: string | null // per-connection terminal scheme; null = global default
  font: string | null
  fontSize: number | null
  logging: boolean // auto-save this host's terminal output to a log file
}

export function blankHost(): Host {
  return {
    id: crypto.randomUUID(),
    name: '',
    hostname: '',
    port: 22,
    user: '',
    protocol: 'ssh',
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
    scheme: null,
    font: null,
    fontSize: null,
    logging: false,
  }
}

// ---- Port forwards --------------------------------------------------------
export type Forward = {
  id: string
  name: string
  hostId: string
  kind: 'local' | 'remote' | 'dynamic'
  bindAddr: string
  bindPort: number
  destHost: string
  destPort: number
}

export function blankForward(): Forward {
  return {
    id: crypto.randomUUID(),
    name: '',
    hostId: '',
    kind: 'local',
    bindAddr: '127.0.0.1',
    bindPort: 8080,
    destHost: '',
    destPort: 0,
  }
}

// list = saved configs; status = live per-forward state (not persisted).
export const forwardsStore = $state({
  list: [] as Forward[],
  status: {} as Record<string, { state: string; message?: string }>,
})

export async function loadForwards() {
  forwardsStore.list = await invoke<Forward[]>('forwards_list')
}
export async function saveForward(f: Forward) {
  const saved = await invoke<Forward>('forward_save', { forward: f })
  const i = forwardsStore.list.findIndex((x) => x.id === saved.id)
  if (i >= 0) forwardsStore.list[i] = saved
  else forwardsStore.list.push(saved)
}
export async function deleteForward(id: string) {
  await invoke('forward_delete', { id })
  forwardsStore.list = forwardsStore.list.filter((f) => f.id !== id)
  delete forwardsStore.status[id]
}
export const startForward = (id: string) => invoke('forward_start', { id })
export const stopForward = (id: string) => invoke('forward_stop', { id })

// forward://state payload router.
export function applyForwardState(id: string, state: string, message?: string) {
  forwardsStore.status[id] = { state, message }
}

// ---- SFTP -----------------------------------------------------------------
export type SftpEntry = { name: string; is_dir: boolean; size: number }
export const openSftp = (hostId: string) =>
  invoke<{ id: string; cwd: string }>('sftp_open', { hostId })
export const sftpList = (id: string, path: string) =>
  invoke<SftpEntry[]>('sftp_list', { id, path })
export const sftpDownload = (id: string, remote: string, local: string) =>
  invoke('sftp_download', { id, remote, local })
export const sftpUpload = (id: string, local: string, remote: string) =>
  invoke('sftp_upload', { id, local, remote })
export const sftpClose = (id: string) => invoke('sftp_close', { id })

// ---- ssh_config import / export (no lock-in) ------------------------------
export const importSshConfig = (path?: string) =>
  invoke<Host[]>('ssh_config_import', { path: path ?? null })
export const importPutty = (path: string) => invoke<Host[]>('putty_import', { path })
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

// ---- Snippets -------------------------------------------------------------
export type Snippet = { id: string; name: string; command: string; confirm: boolean }

export function blankSnippet(): Snippet {
  return { id: crypto.randomUUID(), name: '', command: '', confirm: false }
}

export const snippetsStore = $state({ list: [] as Snippet[] })

export async function loadSnippets() {
  snippetsStore.list = await invoke<Snippet[]>('snippets_list')
}
export async function saveSnippet(s: Snippet) {
  const saved = await invoke<Snippet>('snippet_save', { snippet: s })
  const i = snippetsStore.list.findIndex((x) => x.id === saved.id)
  if (i >= 0) snippetsStore.list[i] = saved
  else snippetsStore.list.push(saved)
}
export async function deleteSnippet(id: string) {
  await invoke('snippet_delete', { id })
  snippetsStore.list = snippetsStore.list.filter((s) => s.id !== id)
}

// Distinct {{var}} tokens in a snippet body, in first-seen order.
export function snippetVars(command: string): string[] {
  const out: string[] = []
  for (const m of command.matchAll(/\{\{\s*(\w+)\s*\}\}/g)) {
    if (!out.includes(m[1])) out.push(m[1])
  }
  return out
}

export function fillSnippet(command: string, values: Record<string, string>): string {
  return command.replace(/\{\{\s*(\w+)\s*\}\}/g, (_, k) => values[k] ?? '')
}

// Send a resolved command to the most-recently-active session (Enter appended).
export function runInActiveSession(command: string): boolean {
  if (!ui.lastSession) return false
  invoke('ssh_write', { id: ui.lastSession, data: command + '\r' })
  return true
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

// ---- Saved views (host-list filters) --------------------------------------
// A view is a named search + tag filter, so only the relevant subset shows.
// UI pref, kept in localStorage (on-device).
export type View = { id: string; name: string; tags: string[]; search: string }
export const viewsStore = $state({
  list: JSON.parse(localStorage.getItem('ssh.views') ?? '[]') as View[],
})
function persistViews() {
  localStorage.setItem('ssh.views', JSON.stringify(viewsStore.list))
}
export function saveView(v: View) {
  const i = viewsStore.list.findIndex((x) => x.id === v.id)
  if (i >= 0) viewsStore.list[i] = v
  else viewsStore.list.push(v)
  persistViews()
}
export function deleteView(id: string) {
  viewsStore.list = viewsStore.list.filter((v) => v.id !== id)
  persistViews()
}

// ---- Nested groups --------------------------------------------------------
// Host.group is a "/"-separated path (e.g. "Clients/Acme"); the sidebar renders
// the derived tree. Selecting a group filters the host list to it + descendants.
export function groupNodes(): { path: string; depth: number; label: string }[] {
  const set = new Set<string>()
  for (const h of store.hosts) {
    if (!h.group) continue
    const parts = h.group.split('/').map((s) => s.trim()).filter(Boolean)
    for (let i = 0; i < parts.length; i++) set.add(parts.slice(0, i + 1).join('/'))
  }
  return [...set]
    .sort()
    .map((p) => ({ path: p, depth: p.split('/').length - 1, label: p.split('/').pop()! }))
}

// Is host `h` inside group path `g` (exact or a descendant)?
export const inGroup = (h: Host, g: string) => !!h.group && (h.group === g || h.group.startsWith(g + '/'))

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
  connLog: LogLine[] // detailed per-step connection log (accordion), current attempt
  activeStep: Step // step the connection is currently in (drives failed-step marking)
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
    connLog: [],
    activeStep: 'connecting',
    keyHost: '',
    fingerprint: '',
    keyType: '',
    keyChanged: false,
    oldFingerprint: '',
  }
}

// `lastSession` = the most-recently-active connected pane's session id (snippet
// "Run" targets it). `group` = selected group-path filter for the host list.
export const ui = $state({
  tabs: [] as Tab[],
  active: 'home' as string,
  lastSession: null as string | null,
  group: null as string | null,
})

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

// ---- Broadcast input ------------------------------------------------------
// Type once, send to many. `exclude` drops session ids from the fan-out; `once`
// auto-exits after a single send. Deliberately loud (see design §9).
export const broadcast = $state({ on: false, exclude: [] as string[], once: false })

// Every currently connected session, across all tabs/panes.
export function connectedSessions(): { id: string; name: string }[] {
  const out: { id: string; name: string }[] = []
  for (const t of ui.tabs)
    for (const p of t.panes)
      if (p.sessionId && p.phase === 'connected')
        out.push({ id: p.sessionId, name: p.host?.name ?? '(host)' })
  return out
}

export const broadcastTargets = () =>
  connectedSessions().filter((s) => !broadcast.exclude.includes(s.id))

export function broadcastLine(text: string) {
  for (const t of broadcastTargets()) invoke('ssh_write', { id: t.id, data: text + '\r' })
  if (broadcast.once) broadcast.on = false
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
    // A fresh 'connecting' begins a new attempt — clear the previous log.
    if (p.state === 'connecting') {
      pane.connLog = []
      pane.activeStep = 'connecting'
    }
    pane.phase = p.state
    pane.error = p.state === 'error' ? (p.message ?? 'error') : ''
    // On error, record the failure under whatever step is active so the
    // accordion shows it inline; activeStep stays put (that's the failed step).
    if (p.state === 'error') pane.connLog.push({ step: pane.activeStep, ts: Date.now(), msg: pane.error })
    const s = phaseStep(p.state)
    if (s) pane.activeStep = s
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

// The ssh://log event payload — one detailed line, bucketed by step.
export type LogPayload = { id: string; step: Step; msg: string }

// Route a detailed connection-log line to its pane and advance the step pointer.
export function applyLog(p: LogPayload) {
  for (const tab of ui.tabs) {
    const pane = tab.panes.find((x) => x.sessionId === p.id)
    if (!pane) continue
    pane.connLog.push({ step: p.step, ts: Date.now(), msg: p.msg })
    pane.activeStep = p.step
    return
  }
}

export const activeCount = () =>
  ui.tabs.reduce((n, t) => n + t.panes.filter((p) => p.phase === 'connected').length, 0)
