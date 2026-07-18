# Modern SSH Client — Requirements v1 (Mac)

Local-first, speed-obsessed SSH client for macOS. Inspirations: mtPuTTY (multi-tab session management) and Termius (modern feature set).

## Guiding Principles

1. **Extremely fast & lightweight** — startup and connection speed are the top priority. Everything is measured against "does this slow the app down."
2. **Local-first** — all data (hosts, keys, config) lives on-device. No account required. Only opt-in anonymous telemetry leaves the machine.
3. **No lock-in** — anything imported can be exported back to standard formats.
4. **Sync is a paid, later tier** — E2E cross-device sync is out of scope for v1.

## Scope

### In scope for v1 (Mac)
SSH, SFTP file transfer, port forwarding (local/remote/dynamic), Mosh, Telnet, jump hosts / ProxyJump, snippets, broadcast input, session logging, reconnect, key management, import/export, theming.

### Out of scope for v1 (paid / later)
- End-to-end encrypted cross-device sync
- Team features: shared vaults, shared keys, collaboration
- Windows build (Mac ships first, Windows follows)

## Layout & Navigation

### Landing page
**Side panel (options):**
1. SSH connection list
2. Key manager
3. History
4. Groups (each group holds N terminals, opened as horizontal tabs on top — one-click to open a whole group)
5. Snippets

Side panel supports **nested folders/groups**, not just a flat list.

**Top bar (tabs):**
1. A pinned main tab listing all sessions — always present, cannot be closed.
2. `+` icon to open a new terminal tab.
3. Each tab can have a **custom color**, shown in the top tab bar (set per-connection at config time).

**Main content (host list view):**
- List/grid of all SSH sessions. Clicking a session opens a new tab and connects.
- Per connection: **name**, **tags** (filterable), **custom icon** (auto-picked by OS/connection type if not set), **recent/favorites** surfaced on the landing page.
- **Multiple saved views** so only the relevant subset of hosts shows.
- **Global fuzzy search** across all hosts + **command palette (Cmd+K)**.

**Footer:** sticky bottom bar (task-bar style) showing active sessions / quick status.

### Live connection stepper
When connecting, show a **live, real** pipeline of steps reflecting the actual connection state — e.g. Connecting → Key (id_rsa) / Password → Verification code (MFA) — driven by the session's real configuration and progress, not a mock animation.

## Core Features

### Terminal & sessions
- **Split terminal** *(separate track — see `docs/splits.md`)*: **drag-and-drop** pane layout (not fixed 1/2/4 buttons), **per-pane close**, and **confirm before disconnecting** any live session. Splits are tab arrangements and **each pane can be a different host**. Pulled out of Phase 1; taken up as its own effort.
- **Terminal groups**: a group holds N terminals shown as horizontal tabs on top; open the whole group with one click.
- **Reconnect / auto-reconnect** on dropped connections *(must-have)*.
- **Find in terminal**: `Cmd+F` opens in-terminal scrollback search *(must-have)*.
- **Broadcast input**: type once, send keystrokes to multiple/all open sessions *(must-have)*.
- **Session logging**: auto-save terminal output per host.

### Connectivity
- SSH, **Mosh** (for flaky links), **Telnet**.
- **Port forwarding** — local, remote, and dynamic/SOCKS — saved as reusable configs *(must-have)*.
- **Jump hosts / bastion / ProxyJump** chaining *(personal must-have)*.
- **SFTP browser** with drag-and-drop file transfer *(must-have)*.

### Productivity
- **Snippets / saved commands** — run scripts or commands with one click, across sessions *(must-have)*.
- **Known-host fingerprint management** with change warnings.

## Key Manager
- Behavior modeled on Termius: import existing keys and **generate keys in-app**.
- Key types: ed25519, RSA, ECDSA, plus **FIDO2 / hardware security key** auth.
- Keys stored securely (see Security).

## Theming
- **Global theme** plus **per-connection theme** (set at configuration time).
- **Per-tab color** shown in the top tab bar.
- Per-connection font (follows the per-connection theme model).

## Import / Export
**Import from:**
- `~/.ssh/config`
- PuTTY sessions (registry / `.reg`)
- `known_hosts`

**Export to:**
- Standard `ssh_config` (so users are never locked in).
- Export of configs and saved connections.

## Security & Privacy
- **macOS Keychain** for credential storage.
- Optional **Touch ID / biometric unlock**.
- **Encrypted local vault** with optional **master-password**.
- **FIDO2 / hardware security key** support.
- **Fully local & anonymous** — nothing but opt-in anonymous telemetry is sent to any server.

## Open Questions / To Decide
1. Telemetry: what exactly is collected, and is it opt-in or opt-out on first run?
2. Snippets: plain command strings only, or parameterized/variable snippets?
3. Session logging: per-host toggle, and where logs are stored / rotation policy?
4. Mosh requires a server-side binary on each host (not purely client-side) — confirm it stays in the v1 connectivity set given that constraint.