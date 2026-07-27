# Connection-step accordion with detailed logs

## Goal

While a connection is being established, show each step as a collapsible
accordion. Opening a step reveals the detailed log lines for that step. On
error, only the failing step is marked failed; earlier steps show succeeded,
later steps stay in their default (pending) state. The user can open any step
to read its logs.

## Steps

Fixed logical steps, driven by real events (never faked):

- SSH: `Connecting` → `Verify host key` → `Authenticate` → `Open shell`
- Telnet: `Connecting` → `Open shell` (no host-key / auth)

A ProxyJump chain stays 4 steps; per-hop connect/verify/auth detail appears as
extra log lines inside the relevant step ("via bastion → host:22", "hop 2/2").

## Backend — `src-tauri/src/ssh.rs`

- Add `emit_log(app, id, step, msg)` emitting `ssh://log` with
  `{ id, step, msg }`. `step ∈ "connecting" | "hostkey" | "auth" | "shell"`.
  No timestamp in payload — the frontend stamps on arrival (IPC-local skew is
  negligible; avoids serializing a clock).
- Breadcrumb calls at existing checkpoints:
  - connecting: `TCP connect {host}:{port}`, `transport established`; per-hop
    `via {prev} -> {host}:{port}` in the jump loop.
  - hostkey: `SHA256:… ({key_type})`, then one of `known host, matched` /
    `unknown key, awaiting confirmation` / `key CHANGED` / `accepted` /
    `rejected`.
  - auth: `authenticate {user}@{host} ({method})`, `authenticated`.
  - shell: `request pty xterm-256color`, `request shell`, `session ready`.
- Failures need no new backend error plumbing: each `.map_err` already yields
  an exact string that becomes `ConnState::Error{message}`. The frontend routes
  that message into the currently-active step and marks it failed. The
  breadcrumbs' real job is to advance the active-step pointer (especially into
  "Open shell", which is not its own `ConnState`) so an error attributes to the
  correct step.

## Frontend

- `state.svelte.ts`: `Pane` gains `connLog: {step, ts, msg}[]` and `activeStep`.
  `applyLog()` pushes a line and sets `activeStep`. A `connecting` state event
  resets `connLog` (new attempt). `connect()` / `reset()` / `reconnect()` clear
  it.
- Rewrite `Stepper.svelte` in place into the accordion (its only consumer).
  Presentational props: `{ phase, error, method, protocol, log, activeStep }`.
  Each step: collapsible header with a status glyph (done / active-spinner /
  failed / future-default) plus a body listing that step's log lines.
  Auto-opens the active-or-failed step; user can toggle any step open.
  Failed step = `activeStep` when `phase === 'error'`; earlier = done, later =
  default.
- `Pane.svelte`: pass the new props to `<Stepper>`; clear `connLog` on
  connect / reset / reconnect.
- Extract step-status into a pure `stepStatus(steps, activeStep, phase)` with
  one assert-based self-check (the only non-trivial branch).

## Deliberate simplifications

1. Log is discarded once connected — the overlay vanishes on success as today;
   no post-connect log-review panel. Add later if inspecting a successful
   trace becomes useful.
2. No backend timestamp; the frontend stamps arrival time.
