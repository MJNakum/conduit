// Runnable check for keybinding parsing: `node src/lib/keys.test.ts`.
import assert from 'node:assert/strict'
import { eventToBinding, formatBinding, isPrintableChord } from './keys.ts'

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

// Printable chords (would type a character) must never be stolen from a field.
assert.equal(isPrintableChord({ key: '?', shiftKey: true }), true) // ? types
assert.equal(isPrintableChord({ key: 'a' }), true)
assert.equal(isPrintableChord({ key: ' ' }), true) // space types
assert.equal(isPrintableChord({ key: 'k', metaKey: true }), false) // Cmd chord
assert.equal(isPrintableChord({ key: 'F6' }), false) // function key
assert.equal(isPrintableChord({ key: 'ArrowDown' }), false)
assert.equal(isPrintableChord({ key: '/', ctrlKey: true }), false)

console.log('keys: ok')
