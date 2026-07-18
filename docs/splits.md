# Split panes — deferred feature (separate track)

Split terminals were prototyped in Phase 1 (slice 3) as fixed 1/2/4-way layouts
with an upfront toolbar. That UX was rejected. Splits are pulled **out of the
Phase 1 acceptance** and treated as their own dedicated effort, to be picked up
after the rest of Phase 1 lands.

The single-pane `Pane` / `Tab` model in `src/lib/state.svelte.ts` already
supports N panes per tab (`tab.panes`, `setLayout`), so this work slots into the
existing state without reshaping it. `TabView.svelte` is where the split UI
lives.

## What was wrong with the prototype

- No per-pane control — can't choose *which* pane to close.
- Shrinking (4→2, 2→1) silently tore down sessions with no confirmation.
- Upfront 1/2/4 layout buttons look bad and are the wrong mental model.

## Requirements for the real thing

1. **Drag-and-drop layout** — arrange panes by dragging a terminal into a
   position (split left/right/top/bottom, or into an existing pane), not a
   fixed layout picker. Layout emerges from where terminals are dropped.
2. **Per-pane close** — each pane has its own close control; closing one pane
   never ambiguously affects others.
3. **Confirm before killing sessions** — any action that would disconnect one or
   more live sessions (closing a pane, collapsing a layout) warns first, naming
   how many sessions will drop.
4. **Each pane is its own host/session** (already true in the model).
5. Focus routing + active-pane highlight (already stubbed via `tab.active`).

## Status

- Prototype toolbar removed; tabs render a single pane for now.
- Pane/Tab state model retained as the foundation.
- Build order for this track (DnD engine, drop zones, close/confirm flow) to be
  planned when the track starts.
