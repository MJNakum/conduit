// Runnable check for the step-status logic: `node src/lib/connsteps.test.ts`.
// No framework — plain asserts. Fails loudly if the branching regresses.
import assert from 'node:assert/strict'
import { SSH_STEPS, formatLog, stepStatus, stepsFor, type LogLine } from './connsteps.ts'

// Mid-connection at auth: connect+hostkey done, auth active, shell pending.
assert.deepEqual(stepStatus(SSH_STEPS, 'auth', 'authenticating'), [
  'done',
  'done',
  'active',
  'pending',
])

// Error during auth: only auth failed; earlier done, later untouched.
assert.deepEqual(stepStatus(SSH_STEPS, 'auth', 'error'), [
  'done',
  'done',
  'failed',
  'pending',
])

// Error on the very first step: it fails, nothing before it.
assert.deepEqual(stepStatus(SSH_STEPS, 'connecting', 'error'), [
  'failed',
  'pending',
  'pending',
  'pending',
])

// Connected: every step done regardless of active pointer.
assert.deepEqual(stepStatus(SSH_STEPS, 'shell', 'connected'), [
  'done',
  'done',
  'done',
  'done',
])

// The MFA step only exists once a server has actually asked for a code.
assert.deepEqual(stepsFor('ssh'), ['connecting', 'hostkey', 'auth', 'shell'])
assert.deepEqual(stepsFor('ssh', true), ['connecting', 'hostkey', 'auth', 'mfa', 'shell'])
assert.deepEqual(stepsFor('telnet', true), ['connecting', 'shell'])

// Awaiting a verification code: auth is behind us, mfa is where we're stuck.
assert.deepEqual(stepStatus(stepsFor('ssh', true), 'mfa', 'keyboard'), [
  'done',
  'done',
  'done',
  'active',
  'pending',
])

// A wrong code fails on the MFA step, not on auth.
assert.deepEqual(stepStatus(stepsFor('ssh', true), 'mfa', 'error'), [
  'done',
  'done',
  'done',
  'failed',
  'pending',
])

// A step that isn't in this protocol's chain must not blank out the stepper.
assert.deepEqual(stepStatus(stepsFor('telnet'), 'auth', 'error'), ['failed', 'pending'])

// formatLog renders both attempts and stamps every line with its step.
{
  const meta = { host: 'me@box:22', version: '0.2.0' }
  const line = (msg: string): LogLine => ({ step: 'auth', ts: 0, msg })
  const text = formatLog([line('server offers: keyboard-interactive')], meta, [line('rejected')])
  assert.match(text, /# Conduit 0\.2\.0 connection log/)
  assert.match(text, /# me@box:22/)
  assert.match(text, /## previous attempt/)
  assert.match(text, /\[auth\] {2}rejected/)
  assert.match(text, /\[auth\] {2}server offers: keyboard-interactive/)
  // No previous attempt -> no divider to explain away.
  assert.doesNotMatch(formatLog([line('x')], meta), /## previous attempt/)
}

console.log('connsteps: ok')
