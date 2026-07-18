# CLAUDE.md

Always-loaded context for this repo. Read `docs/requirements.md` for the full spec and `docs/mvp-plan.md` for the build order before starting any feature.

## Commands

- `pnpm tauri dev` — run the app (starts Vite on :1420 + the Rust shell).
- `pnpm tauri build` — production bundle.
- `pnpm check` — Svelte/TS type-check.
- `pnpm build` — frontend-only bundle to `dist/`.
- `cargo build` / `cargo test` — run in `src-tauri/`.

Rust lives in `src-tauri/` (SSH + PTY, `russh`); frontend in `src/` (Svelte 5 + xterm.js, presentation only). SSH streams to the webview over the `ssh://data` / `ssh://state` Tauri events — never polling.

## What we're building

A **modern, local-first SSH client for macOS**. Inspirations: mtPuTTY (multi-tab session management) and Termius (modern feature set). Windows follows after Mac ships.

## Non-negotiable principles

1. **Speed & lightness above all.** Startup time and connection latency are the top metric. Reject dependencies, abstractions, or patterns that add meaningful weight. When choosing between "fast" and "clever," choose fast.
2. **Local-first.** All data (hosts, keys, config, logs) lives on-device. No account required to use the app. Nothing leaves the machine except opt-in anonymous telemetry.
3. **No lock-in.** Anything imported must be exportable back to standard formats (`ssh_config`).
4. **Sync/team/Windows are out of scope for v1.** Do not build them. They are reserved for a later paid tier.

## Tech stack (decided — do not change without asking)

- **Shell:** Tauri (Rust backend + web frontend). Chosen for small binary size, low memory, and a cleaner Windows port later.
- **Terminal rendering:** xterm.js in the webview.
- **SSH/transport:** Rust side. Use `russh` (async, pure-Rust) for the SSH implementation; bridge sessions to the frontend over Tauri commands/events. Confirm exact crate choices in the first planning step before writing connection code.
- **Frontend framework:** keep it light. Prefer a minimal reactive setup over a heavy SPA framework. Propose the choice in Phase 0 and wait for sign-off.
- **Credential storage:** macOS Keychain (via a Rust keychain crate). Never store secrets in plaintext, app config, or logs.

## Architecture notes

- Terminal I/O and SSH transport run in Rust; the webview is presentation only. Stream PTY data over Tauri events, not polling.
- The **live connection stepper** must reflect *real* connection state (Connecting → Key/Password → MFA), driven by actual events from the Rust side — never a faked animation.
- Splits and groups are tab arrangements; **each pane/tab can be a different host**.
- Design the session layer so **broadcast input** (one keystroke → many sessions) and **jump-host/ProxyJump chaining** are possible from day one, even if built later. These are core, not bolt-ons.

## Conventions

- Small, reviewable commits scoped to one feature.
- No secrets in code, logs, or error messages. Redact host credentials in any diagnostic output.
- Prefer standard SSH file formats on disk so import/export stays trivial.
- Ask before adding a new dependency of nontrivial size; note the weight tradeoff.
- **No emoji in the UI or code.** Use icons from `@lucide/svelte` (already a dep). This applies everywhere — host icons, tab bar, buttons, status glyphs.

## How to work with me

- Build **one phase at a time** per `docs/mvp-plan.md`. Do not scaffold the entire app in one pass.
- At the start of each phase, restate the plan and the crates/libs you'll use, then wait for confirmation before implementing.
- Flag anything that conflicts with the "speed & lightness" principle.