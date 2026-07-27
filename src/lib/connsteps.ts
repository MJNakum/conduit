// Pure connection-step logic, shared by Stepper.svelte and state.svelte.ts.
// Kept free of Svelte runes so it stays unit-testable under plain `node`.

export type Step = 'connecting' | 'hostkey' | 'auth' | 'shell'
export type StepState = 'done' | 'active' | 'failed' | 'pending'
export type LogLine = { step: Step; ts: number; msg: string }

// Fixed step order per protocol. Telnet has no host-key / auth exchange.
export const SSH_STEPS: Step[] = ['connecting', 'hostkey', 'auth', 'shell']
export const TELNET_STEPS: Step[] = ['connecting', 'shell']

export const STEP_LABEL: Record<Step, string> = {
  connecting: 'Connecting',
  hostkey: 'Verify host key',
  auth: 'Authenticate',
  shell: 'Open shell',
}

export function stepsFor(protocol: string): Step[] {
  return protocol === 'telnet' ? TELNET_STEPS : SSH_STEPS
}

// Map a real ssh://state phase onto the step it belongs to, so state
// transitions advance the active-step pointer alongside the log breadcrumbs.
export function phaseStep(phase: string): Step | null {
  if (phase === 'connecting') return 'connecting'
  if (phase === 'hostkey') return 'hostkey'
  if (phase === 'authenticating') return 'auth'
  return null // connected/error/disconnected keep the last known step
}

// Per-step status aligned to `steps`. Failed lands only on the active step; the
// earlier steps read done, the later ones stay pending (the spec's rule).
export function stepStatus(steps: Step[], activeStep: Step, phase: string): StepState[] {
  const ai = steps.indexOf(activeStep)
  return steps.map((_, i) => {
    if (phase === 'connected' || phase === 'disconnected') return 'done'
    if (phase === 'error') return i < ai ? 'done' : i === ai ? 'failed' : 'pending'
    // in progress
    return i < ai ? 'done' : i === ai ? 'active' : 'pending'
  })
}
