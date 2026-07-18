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
- 2-way and 4-way splits, each pane a potentially different host.
- Live connection stepper driven by real connection events.
- Reconnect / auto-reconnect on drop.
- `Cmd+F` in-terminal search.
- Global fuzzy search + `Cmd+K` command palette.
- **Exit criteria:** can save, organize, open, and reconnect real hosts with split panes.

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