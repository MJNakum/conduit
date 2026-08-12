// Pure connection-step logic, shared by Stepper.svelte and state.svelte.ts.
// Kept free of Svelte runes so it stays unit-testable under plain `node`.

export type Step = 'connecting' | 'hostkey' | 'auth' | 'mfa' | 'shell'
export type StepState = 'done' | 'active' | 'failed' | 'pending'
export type LogLine = { step: Step; ts: number; msg: string }

// Fixed step order per protocol. Telnet has no host-key / auth exchange.
export const SSH_STEPS: Step[] = ['connecting', 'hostkey', 'auth', 'shell']
// Same chain plus the verification-code step, used once a server asks for one.
export const SSH_MFA_STEPS: Step[] = ['connecting', 'hostkey', 'auth', 'mfa', 'shell']
export const TELNET_STEPS: Step[] = ['connecting', 'shell']

export const STEP_LABEL: Record<Step, string> = {
  connecting: 'Connecting',
  hostkey: 'Verify host key',
  auth: 'Authenticate',
  mfa: 'Verification code',
  shell: 'Open shell',
}

// `mfa` is set once the server has actually issued a keyboard-interactive
// challenge — only the steps that apply to this host appear (design-spec §5),
// so hosts without a second factor never show an MFA row.
export function stepsFor(protocol: string, mfa = false): Step[] {
  if (protocol === 'telnet') return TELNET_STEPS
  return mfa ? SSH_MFA_STEPS : SSH_STEPS
}

// Map a real ssh://state phase onto the step it belongs to, so state
// transitions advance the active-step pointer alongside the log breadcrumbs.
export function phaseStep(phase: string): Step | null {
  if (phase === 'connecting') return 'connecting'
  if (phase === 'hostkey') return 'hostkey'
  if (phase === 'authenticating') return 'auth'
  if (phase === 'keyboard') return 'mfa'
  return null // connected/error/disconnected keep the last known step
}

// HH:MM:SS.mmm — the arrival time the frontend stamped on a log line.
export function stampTime(ts: number): string {
  const d = new Date(ts)
  return d.toTimeString().slice(0, 8) + '.' + String(d.getMilliseconds()).padStart(3, '0')
}

export type LogMeta = { host: string; version: string }

// Render the whole attempt as one plain-text block for Copy / Save. Contains
// only lines the backend already emitted, which carry prompt text but never an
// answer or a secret (CLAUDE.md) — so this is safe to put on the clipboard or
// on disk. `previous` is the prior attempt, kept so a retry doesn't destroy the
// evidence of what just failed.
export function formatLog(lines: LogLine[], meta: LogMeta, previous: LogLine[] = []): string {
  const body = (ls: LogLine[]) =>
    ls.map((l) => `${stampTime(l.ts)}  [${l.step}]  ${l.msg}`).join('\n')
  const out = [`# Conduit ${meta.version} connection log`, `# ${meta.host}`]
  if (previous.length) {
    out.push('', '## previous attempt', body(previous))
  }
  out.push('', `## attempt at ${lines.length ? stampTime(lines[0].ts) : '(no lines)'}`, body(lines))
  return out.join('\n') + '\n'
}

// Per-step status aligned to `steps`. Failed lands only on the active step; the
// earlier steps read done, the later ones stay pending (the spec's rule).
export function stepStatus(steps: Step[], activeStep: Step, phase: string): StepState[] {
  // A step outside this chain (telnet never has 'auth') would otherwise index to
  // -1 and render every row pending, which reads as a broken stepper. Treat it
  // as the first step instead.
  const ai = Math.max(0, steps.indexOf(activeStep))
  return steps.map((_, i) => {
    if (phase === 'connected' || phase === 'disconnected') return 'done'
    if (phase === 'error') return i < ai ? 'done' : i === ai ? 'failed' : 'pending'
    // in progress
    return i < ai ? 'done' : i === ai ? 'active' : 'pending'
  })
}
