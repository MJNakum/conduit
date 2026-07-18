# MVP Build Plan

Build **one phase at a time**. At the start of each phase, restate the plan and the crates/libraries you intend to use, then wait for confirmation before implementing. Do not scaffold the whole app up front.

## Phase 0 — Foundation & decisions
- Initialize the Tauri project (Rust backend + web frontend).
- Propose and confirm: frontend framework (keep it light), SSH crate (`russh` unless a better fit is argued), Keychain crate, xterm.js integration approach.
- Establish the session architecture: PTY/SSH I/O in Rust, streamed to the webview over Tauri events. Presentation layer stays dumb.
- Set up the host-config data model on disk in a format close to `ssh_config` to keep import/export trivial.
- **Exit criteria:** a window opens, a single hardcoded SSH connection streams live terminal output through xterm.js.

## Phase 1 — Core terminal experience (MVP)
- Host list view: name, tags (filterable), custom/auto icon, recent/favorites.
- Tabbed terminals + pinned "all sessions" main tab that can't close.
- ~~2-way and 4-way splits~~ → **deferred to a separate track** (drag-and-drop layout, per-pane close, confirm-before-disconnect). See `docs/splits.md`. Not part of Phase 1 acceptance.
- Live connection stepper driven by real connection events.
- Reconnect / auto-reconnect on drop.
- `Cmd+F` in-terminal search.
- Global fuzzy search + `Cmd+K` command palette.
- **App shell + design system** (added in the design pass after `docs/design-spec.md` / `docs/ssh-client-mockup.html`): the shared CSS-variable token system (`src/app.css` `:root`, dark-first — see design-spec §1.1), the left sidebar frame + always-visible vault-status pill, the slim global footer, and the status-dot language (green connected / blue connecting / red disconnected / grey idle) with the host accent color as a separate 2px tab top-rule + host-row rail. Sidebar sections beyond **Hosts** (Keys, Snippets, Port Forwards, History, the Groups tree) render as **inert dimmed placeholders** — the shell reserves their space, but each is owned and wired up by its own later phase.
- **Exit criteria:** can save, organize, open, and reconnect real hosts in tabs, in a shell that matches the design spec. (Splits moved to their own track — `docs/splits.md`.)

## Phase 2 — Auth, keys, security (MVP)
- Key manager: import + in-app generation (ed25519, RSA, ECDSA).
- macOS Keychain storage + Touch ID unlock.
- Jump host / ProxyJump chaining.
- Known-host fingerprint management with change warnings.
- **Exit criteria:** connect via key or password through a bastion, secrets only in Keychain.

## Phase 3 — Theming & import/export (MVP)
- Global + per-connection theme, per-tab color.
- Import from `~/.ssh/config`; export to standard `ssh_config`.
- **Exit criteria:** an existing `~/.ssh/config` imports cleanly and round-trips back out.

--- MVP COMPLETE ABOVE THIS LINE ---

## Phase 4 — Fast follow (post-MVP)
SFTP browser (drag-and-drop), snippets, broadcast input, port forwarding (local/remote/dynamic), session logging, Mosh/Telnet, PuTTY import, nested groups, saved views.

## Later / paid tier
E2E encrypted sync, team/shared vaults, Windows build, FIDO2/hardware key auth, encrypted local vault + master password.