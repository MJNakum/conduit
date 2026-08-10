// Runnable check for keybinding parsing: `node src/lib/keys.test.ts`.
import assert from 'node:assert/strict'
import { eventToBinding, formatBinding, isPrintableChord } from './keys.ts'

// ── macOS behaviour (mac = true) ──────────────────────────────────────────────

// Cmd (metaKey) → 'mod'; modifiers normalize in fixed order.
assert.equal(eventToBinding({ key: 'T', metaKey: true, shiftKey: true }, true), 'mod+shift+t')
assert.equal(eventToBinding({ key: 'k', metaKey: true }, true), 'mod+k')
assert.equal(eventToBinding({ key: ',', metaKey: true }, true), 'mod+,')
assert.equal(eventToBinding({ key: 'ArrowRight', metaKey: true, shiftKey: true }, true), 'mod+shift+arrowright')

// Bare modifier presses produce no binding.
assert.equal(eventToBinding({ key: 'Meta', metaKey: true }, true), null)
assert.equal(eventToBinding({ key: 'Shift', shiftKey: true }, true), null)

// Pretty-print uses macOS glyphs, no separator.
assert.equal(formatBinding('mod+k', true), '⌘K')
assert.equal(formatBinding('mod+shift+t', true), '⌘⇧T')
assert.equal(formatBinding('mod+shift+arrowright', true), '⌘⇧→')
assert.equal(formatBinding('mod+1', true), '⌘1')

// ── Windows/Linux behaviour (mac = false) ────────────────────────────────────

// Ctrl (ctrlKey) → 'mod'.
assert.equal(eventToBinding({ key: 'k', ctrlKey: true }, false), 'mod+k')
assert.equal(eventToBinding({ key: 'T', ctrlKey: true, shiftKey: true }, false), 'mod+shift+t')
assert.equal(eventToBinding({ key: ',', ctrlKey: true }, false), 'mod+,')

// Pretty-print uses text labels with '+' separator.
assert.equal(formatBinding('mod+k', false), 'Ctrl+K')
assert.equal(formatBinding('mod+shift+t', false), 'Ctrl+Shift+T')
assert.equal(formatBinding('mod+shift+arrowright', false), 'Ctrl+Shift+→')
assert.equal(formatBinding('mod+1', false), 'Ctrl+1')

// ── Printable chord detection (platform-independent) ─────────────────────────

assert.equal(isPrintableChord({ key: '?', shiftKey: true }), true) // ? types
assert.equal(isPrintableChord({ key: 'a' }), true)
assert.equal(isPrintableChord({ key: ' ' }), true) // space types
assert.equal(isPrintableChord({ key: 'k', metaKey: true }), false) // Cmd chord
assert.equal(isPrintableChord({ key: 'F6' }), false) // function key
assert.equal(isPrintableChord({ key: 'ArrowDown' }), false)
assert.equal(isPrintableChord({ key: '/', ctrlKey: true }), false)

console.log('keys: ok')
