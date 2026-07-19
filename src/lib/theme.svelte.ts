// Theming (design-spec §18). Two separate color concerns, kept apart:
//  - the *app* theme (dark/light/system) drives the chrome via CSS tokens;
//  - a *terminal scheme* (bg/fg/cursor + 16 ANSI) is the reading surface.
// Global prefs + custom schemes live in localStorage (UI prefs, on-device).
// Per-connection scheme/font/size live on the Host (hosts.json).

export type Scheme = {
  name: string
  background: string
  foreground: string
  cursor: string
  // 16 ANSI: black,red,green,yellow,blue,magenta,cyan,white, then the bright 8.
  ansi: string[]
  builtin?: boolean
}

// A small, recognizable built-in set. Values are the schemes' canonical hexes.
export const BUILTINS: Scheme[] = [
  {
    name: 'Default Dark',
    background: '#0a0e13',
    foreground: '#d6dde6',
    cursor: '#22c58b',
    ansi: [
      '#0a0e13', '#e5484d', '#22c58b', '#f5c451', '#3b82f6', '#a855f7', '#22b8cf', '#c1c8d1',
      '#3b444f', '#ff6369', '#4ade80', '#fde047', '#60a5fa', '#c084fc', '#67e8f9', '#f0f4f8',
    ],
  },
  {
    name: 'Dracula',
    background: '#282a36',
    foreground: '#f8f8f2',
    cursor: '#f8f8f2',
    ansi: [
      '#21222c', '#ff5555', '#50fa7b', '#f1fa8c', '#bd93f9', '#ff79c6', '#8be9fd', '#f8f8f2',
      '#6272a4', '#ff6e6e', '#69ff94', '#ffffa5', '#d6acff', '#ff92df', '#a4ffff', '#ffffff',
    ],
  },
  {
    name: 'Nord',
    background: '#2e3440',
    foreground: '#d8dee9',
    cursor: '#d8dee9',
    ansi: [
      '#3b4252', '#bf616a', '#a3be8c', '#ebcb8b', '#81a1c1', '#b48ead', '#88c0d0', '#e5e9f0',
      '#4c566a', '#bf616a', '#a3be8c', '#ebcb8b', '#81a1c1', '#b48ead', '#8fbcbb', '#eceff4',
    ],
  },
  {
    name: 'Solarized Dark',
    background: '#002b36',
    foreground: '#839496',
    cursor: '#93a1a1',
    ansi: [
      '#073642', '#dc322f', '#859900', '#b58900', '#268bd2', '#d33682', '#2aa198', '#eee8d5',
      '#002b36', '#cb4b16', '#586e75', '#657b83', '#839496', '#6c71c4', '#93a1a1', '#fdf6e3',
    ],
  },
  {
    name: 'Solarized Light',
    background: '#fdf6e3',
    foreground: '#657b83',
    cursor: '#586e75',
    ansi: [
      '#073642', '#dc322f', '#859900', '#b58900', '#268bd2', '#d33682', '#2aa198', '#eee8d5',
      '#002b36', '#cb4b16', '#586e75', '#657b83', '#839496', '#6c71c4', '#93a1a1', '#fdf6e3',
    ],
  },
  {
    name: 'Gruvbox Dark',
    background: '#282828',
    foreground: '#ebdbb2',
    cursor: '#ebdbb2',
    ansi: [
      '#282828', '#cc241d', '#98971a', '#d79921', '#458588', '#b16286', '#689d6a', '#a89984',
      '#928374', '#fb4934', '#b8bb26', '#fabd2f', '#83a598', '#d3869b', '#8ec07c', '#ebdbb2',
    ],
  },
].map((s) => ({ ...s, builtin: true }))

const DEFAULT_FONT = 'ui-monospace, "SF Mono", "JetBrains Mono", monospace'

function ls(key: string, fallback: string): string {
  return localStorage.getItem(key) ?? fallback
}

export type AppTheme = 'dark' | 'light' | 'system'

// Global prefs (reactive). Read directly in components; mutate via the setters
// below so changes persist and re-apply.
export const settings = $state({
  appTheme: ls('ssh.appTheme', 'system') as AppTheme,
  defaultScheme: ls('ssh.defaultScheme', 'Default Dark'),
  defaultFont: ls('ssh.defaultFont', DEFAULT_FONT),
  defaultFontSize: Number(localStorage.getItem('ssh.defaultFontSize')) || 13,
})

// User-created schemes, editable and exportable.
export const customSchemes = $state({
  list: JSON.parse(localStorage.getItem('ssh.customSchemes') ?? '[]') as Scheme[],
})

export const allSchemes = (): Scheme[] => [...BUILTINS, ...customSchemes.list]
export const schemeByName = (name: string): Scheme =>
  allSchemes().find((s) => s.name === name) ?? BUILTINS[0]

// The effective scheme for a host: its own, else the global default.
export function resolveScheme(scheme: string | null | undefined): Scheme {
  return schemeByName(scheme || settings.defaultScheme)
}

// Map a Scheme to an xterm ITheme.
export function xtermTheme(s: Scheme) {
  const a = s.ansi
  return {
    background: s.background,
    foreground: s.foreground,
    cursor: s.cursor,
    cursorAccent: s.background,
    black: a[0], red: a[1], green: a[2], yellow: a[3],
    blue: a[4], magenta: a[5], cyan: a[6], white: a[7],
    brightBlack: a[8], brightRed: a[9], brightGreen: a[10], brightYellow: a[11],
    brightBlue: a[12], brightMagenta: a[13], brightCyan: a[14], brightWhite: a[15],
  }
}

// ---- App theme (chrome) ---------------------------------------------------
const mql = window.matchMedia('(prefers-color-scheme: dark)')
const resolved = (t: AppTheme): 'dark' | 'light' =>
  t === 'system' ? (mql.matches ? 'dark' : 'light') : t

export function applyAppTheme() {
  const r = resolved(settings.appTheme)
  document.documentElement.setAttribute('data-theme', r)
  document.documentElement.style.colorScheme = r
}
// Follow the OS when in system mode.
mql.addEventListener('change', () => {
  if (settings.appTheme === 'system') applyAppTheme()
})

// ---- Setters (persist + apply) --------------------------------------------
export function setAppTheme(t: AppTheme) {
  settings.appTheme = t
  localStorage.setItem('ssh.appTheme', t)
  applyAppTheme()
}
export function saveDefaults() {
  localStorage.setItem('ssh.defaultScheme', settings.defaultScheme)
  localStorage.setItem('ssh.defaultFont', settings.defaultFont)
  localStorage.setItem('ssh.defaultFontSize', String(settings.defaultFontSize))
}
export function saveCustomSchemes() {
  localStorage.setItem('ssh.customSchemes', JSON.stringify(customSchemes.list))
}
