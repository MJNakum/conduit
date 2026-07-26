// Runnable check for keybinding parsing: `node src/lib/keys.test.ts`.
import assert from 'node:assert/strict'
import { eventToBinding, formatBinding } from './keys.ts'

// Modifiers normalize in fixed order regardless of how the event reports them.
assert.equal(eventToBinding({ key: 'T', metaKey: true, shiftKey: true }), 'mod+shift+t')
assert.equal(eventToBinding({ key: 'k', metaKey: true }), 'mod+k')
assert.equal(eventToBinding({ key: ',', metaKey: true }), 'mod+,')
assert.equal(eventToBinding({ key: 'ArrowRight', metaKey: true, shiftKey: true }), 'mod+shift+arrowright')

// Bare modifier presses produce no binding.
assert.equal(eventToBinding({ key: 'Meta', metaKey: true }), null)
assert.equal(eventToBinding({ key: 'Shift', shiftKey: true }), null)

// Pretty-print maps to platform glyphs.
assert.equal(formatBinding('mod+k'), '⌘K')
assert.equal(formatBinding('mod+shift+t'), '⌘⇧T')
assert.equal(formatBinding('mod+shift+arrowright'), '⌘⇧→')
assert.equal(formatBinding('mod+1'), '⌘1')

console.log('keys: ok')
