// Runnable check for the step-status logic: `node src/lib/connsteps.test.ts`.
// No framework — plain asserts. Fails loudly if the branching regresses.
import assert from 'node:assert/strict'
import { SSH_STEPS, stepStatus } from './connsteps.ts'

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

console.log('connsteps: ok')
