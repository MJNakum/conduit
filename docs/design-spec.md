# Modern SSH Client — Design Spec

Dark-first, dense, keyboard-driven. Built for shadcn-svelte. macOS, local-first.

---

## 0. Who this is for, and what that means for the design

The user is a developer, sysadmin, or DevOps engineer who lives in the terminal and juggles anywhere from a handful to several hundred hosts. They open the app dozens of times a day, usually to do one thing fast: find a machine and get a shell. They are keyboard-fluent, allergic to latency, and suspicious of anything that feels like a "cloud product" bolted onto their tools.

Four design commitments fall out of that, and every screen below is measured against them:

**Speed is a feature you can see.** The app opens to the host list already focused and searchable — no splash, no "loading your workspace." The connection stepper shows *real* connection state, not a decorative spinner, because this audience can tell the difference and trusts the honest one. Nothing animates longer than it takes to be understood (120–180ms).

**The keyboard is the primary input; the mouse is optional.** `Cmd+K` is the spine of the app — any host, any action, any setting is reachable from it. Every list is arrow-navigable, every destructive action is confirmable with `Enter`, and shortcuts are shown inline (not hidden in a cheat sheet) so they're learned by osmosis.

**Density with hierarchy, not density as noise.** Power users want to see 20+ hosts at once. We get there with compact 32px rows, a monospace accent for machine-readable data (host:port, fingerprints, key types), and restraint on color — color means something here (tag, status, tab identity), so it's never decorative.

**Local-first is a posture, not just a storage detail.** There is no account, no sign-in wall, no "sync to continue." The one piece of chrome we *do* surface is trust state: vault locked/unlocked, where a secret lives (Keychain), whether a host's fingerprint changed. Those are the things this user actually wants reassurance about.

---

## 1. Design system foundations

### 1.1 Theme & color tokens (shadcn-svelte / Tailwind CSS variables)

shadcn-svelte themes through CSS custom properties on `:root` and `.dark`. We ship **dark as the default** (`.dark` applied at the root) with a first-class light theme. Tokens use the shadcn naming so every component inherits them for free.

Core semantic tokens (dark). These are the **shipped values** (`src/app.css` `:root`, stored as bare HSL triplets so components compose them as `hsl(var(--token) / <alpha>)`). Dark ships first; a `.light` block slots in at Phase 3.

| Token | Value (dark) | Use |
|---|---|---|
| `--background` | `222 16% 8%` | app canvas |
| `--card` | `224 13% 11%` | sidebar, panels, host rows |
| `--popover` | `224 14% 10%` | command palette, menus |
| `--muted` | `224 10% 15%` | hover fills, chips |
| `--muted-foreground` | `220 8% 58%` | secondary text, metadata |
| `--foreground` | `210 18% 90%` | primary text |
| `--border` | `224 10% 17%` | hairlines, row dividers |
| `--primary` | `160 84% 42%` | connect/confirm, focus ring accent |
| `--primary-foreground` | `224 14% 8%` | text on primary |
| `--destructive` | `0 72% 55%` | disconnect, fingerprint-changed, delete |
| `--ring` | `160 84% 42%` | keyboard focus outline |
| `--connecting` | `210 90% 58%` | status dot: connecting / reconnecting |
| `--amber` | `38 92% 55%` | broadcast-caution banner, favorite star |
| `--violet` | `265 70% 66%` | remote port-forward badge |
| `--radius` | `8px` | default corner radius |

The `--primary` green reads as "terminal / go / connected" without being a generic SaaS blue. It's used sparingly: the connect button, the active-session dot, the focused command-palette row.

**Status colors** (a fixed, learnable status language — carried by a single status dot everywhere a session appears: tabs, footer, host rows, pane headers):

- Connected — green `--primary`
- Connecting / reconnecting — blue `hsl(210 90% 58%)`
- Disconnected / dropped — red `--destructive`
- Idle / saved (never yet connected this session) — neutral `--muted-foreground`

The status dot is **only** ever driven by connection state — it is deliberately decoupled from a connection's chosen accent color, so a glance at the dot always answers one question ("is this alive?") and nothing else. Amber is reserved exclusively for the broadcast-caution state (§9); it is never a connection status.

**Per-tab / per-connection accent colors** are a *separate* palette (8 presets + custom) and are the only place color is user-chosen. They appear as a thin 2px top rule on the tab and a subtle rail on the host row — never as a full fill, and never as the dot — so identity-color and status-color occupy different visual channels and can't be confused.

### 1.2 Typography

- **UI sans:** Inter (system-ui fallback) — labels, buttons, body.
- **Mono:** JetBrains Mono / SF Mono — the terminal itself, plus every machine string in the UI (host addresses, ports, fingerprints, key IDs, IPs). Mono for machine data is a deliberate signal: if it's monospace, it's a literal value you might copy.
- Scale: `12px` metadata, `13px` body/rows (default), `14px` section titles, `16px`/`18px` only for empty-state headers. Dense but not cramped; line-height 1.4 on rows.

### 1.3 Density & spacing

- Base unit 4px. Host rows are **32px** tall (comfortable) with a **28px "compact" toggle** for people with 200+ hosts.
- Sidebar 240px (collapsible to 56px icon rail). Right-hand inspector/editor 360px.
- Content max-width is *not* constrained — terminals and host grids use the full window.

### 1.4 Iconography

Lucide stroke icons only (ships with shadcn-svelte) — **no emoji anywhere in the product**; every glyph is a proper monochrome line icon that inherits `currentColor`. Auto-icon logic: if the user doesn't set a custom icon, we pick a Lucide icon from OS/connection hints — a database icon for known DB ports, a router/radio icon for network gear, a monitor for local machines, a shield for bastions/jump hosts, a generic server otherwise. Icons are 16px, `--muted-foreground` at rest. Note the host-row icon reflects host *type* and stays neutral; connection *status* is carried by the separate status dot, not by tinting the icon — one signal per element.

### 1.5 Motion

Motion is functional only: connection-stepper progress, tab open/close (140ms slide+fade), palette open (120ms scale-from-98%), drop-zone highlight on split drag. No idle/ambient animation. `prefers-reduced-motion` collapses all of it to instant.

### 1.6 Core shadcn-svelte components in play

`Command` (palette + fuzzy search), `Dialog` / `AlertDialog` (editor, confirms), `Sheet` (SFTP transfer tray, quick inspector), `Tabs`, `Resizable` (splits, sidebar/inspector), `Tooltip`, `DropdownMenu` / `ContextMenu` (row + tab actions), `Popover`, `Badge` (tags/status), `Input` / `Form` (config), `Select`, `Switch`, `Table` (SFTP, known-hosts, port-forwards), `Tree` (nested groups — via a small custom recursive component over `Collapsible`), `Toast` (`svelte-sonner`), `ScrollArea`, `Separator`, `Avatar` (host icon), `Progress` (stepper).

---

## 2. App shell / global layout

Three persistent regions frame everything. This is the frame the user never leaves.

**Left sidebar (240px, `Resizable` + collapsible).** A vertical section switcher, not a host list itself — the sections are: **Hosts**, **Keys**, **Snippets**, **Port Forwards**, **History**, plus a **Groups/Folders tree** that can nest arbitrarily. The tree is the primary navigation for people who organize by project/client/environment. A collapsed 56px icon-rail mode is available for maximum terminal real estate; it keeps the same sections as icons with tooltips.

At the very bottom of the sidebar sits the **vault status pill** — a lock icon plus "Vault unlocked" / "Locked — Touch ID to unlock." This is deliberately always-visible; it's the local-first trust anchor.

**Top tab bar.** The first tab is a **pinned "Sessions" tab** (a home/dashboard tab) that cannot be closed — it always shows the host list + live-session overview. To its right, one tab per open terminal (or group, or split-tab). A `+` opens a new tab to the host picker. Each tab carries: a **status dot** (green connected / blue connecting / red disconnected — status only, never the accent color), the host name, the connection's accent color as a thin 2px top rule, and a close affordance that's replaced by a confirm if the session is live. Tabs are reorderable by drag. Overflow collapses into an overflow menu with fuzzy jump.

**Footer (sticky, 26px) — slim global status bar, not a session list.** An earlier draft duplicated each session's name here as a dock-style strip; that repeated what the tab bar already shows, so it's cut. The footer now carries only *global* state that has no other home: total active-session count, the current theme, and the `Cmd+K` hint. It's a low-noise status line — the kind of thing you read once in a while, not a second navigation surface. (If even this feels like too much, it can be hidden; nothing else depends on it.) Per-session liveness lives in one place only — the tab bar's status dots — so there's a single source of truth for "what's alive."

**Window chrome.** Native macOS traffic lights; the tab bar sits in the title area (Termius/VS Code style) to save vertical space. Full-window and native-fullscreen supported.

Rationale: navigation lives in the sidebar, the workspace in the tabs, and per-session liveness on the tab dots — each on its own axis, none duplicated. The footer is a thin global readout, not a competing list, which keeps the shell clean and gives status a single source of truth.

---

## 3. Host list view (the pinned "Sessions" tab)

This is the landing surface and the most-used screen. It has to answer "where's my machine?" in under a second.

**Layout.** A slim toolbar on top, the host collection filling the body, and — when any sessions are live — a thin "Active now" strip pinned above the list.

**Toolbar (left→right):** the **saved-view switcher** (a `Select`/segmented control: "All Hosts", "Production", "Clients", + custom views), a **live filter/search field** (focuses on load and on `/`), a **tag filter** (`Popover` with a multi-select tag list; active tags show as removable `Badge`s inline), a **density toggle** (comfortable/compact), and a **view toggle** (list ⇄ grid). Far right: **＋ New Host**.

**Active-now strip.** If sessions are open, a horizontal row of compact cards (name, status dot, latency, uptime) sits at the top so reconnecting to something already running is one click and you never accidentally open a duplicate.

**Host collection — list mode (default).** 32px rows: `[accent rail] [icon] name  ·  user@host:port (mono, muted)  ·  tags  ·  [favorite star] [last-connected, right-aligned muted]`. The whole row is one click to connect (opens a new tab + runs the stepper). Hover reveals a trailing action cluster: connect, connect-in-split, edit, SFTP, `⋯`. Right-click = same as `⋯` (`ContextMenu`). Arrow keys move a selection highlight; `Enter` connects; `Cmd+Enter` connects in a new split; `E` edits.

**Grid mode.** Cards (~200px) for people who navigate visually / by icon — bigger icon, name, tag chips, status. Same interactions.

**Favorites & recent.** Above the full list (when no search is active) two optional collapsible clusters: **Favorites** (starred) and **Recent** (last N connected, from History). These collapse to nothing once the user types a search, so the list doesn't jump around mid-hunt.

**Empty / first-run state.** Big friendly panel: "No hosts yet." Two primary paths — **Add a host** and **Import from ~/.ssh/config** (we detect the file and show a count: "We found 14 hosts in your ~/.ssh/config — import them?"). This is the single most important onboarding moment for this audience; meeting them at their existing config is the fastest possible time-to-value.

**Components:** `Command`-style filtering, `Table` or custom virtualized list (virtualize past ~100 rows for scroll speed), `Badge`, `Select`, `Popover`, `ContextMenu`, `Tooltip`, `Toggle`.

**Why these choices.** The whole-row-connects behavior is the single highest-frequency action, so it gets zero friction; the destructive/less-common actions hide behind hover/right-click so they never slow the common path. Saved views + tag filters exist because at 100+ hosts a flat list is unusable — but they're *optional layers* over a list that works fine empty, so a 5-host user never pays the complexity tax.

---

## 4. Global fuzzy search & command palette (`Cmd+K`)

The keyboard spine. One component, two modes.

**Default mode — jump.** `Cmd+K` (or `Cmd+P`) opens a centered `Command` dialog. Typing fuzzy-matches across **hosts, groups, snippets, keys, and saved views** simultaneously, each result tagged with its kind (`Badge`) and its most useful default action ("↵ connect", "↵ open group"). Results are ranked by frecency (frequency + recency from History). `Enter` runs the default; `Cmd+Enter` runs the alternate (e.g. connect-in-split); `Tab` on a host reveals its sub-actions (SFTP, edit, copy address).

**Action mode — command.** Typing `>` (or the palette naturally surfacing commands when the query matches a verb) switches to app commands: "New host", "Import ssh_config", "Toggle broadcast", "Split right", "Lock vault", "Change theme", "Open port-forward…". Every command shows its keybinding on the right, which is how users learn shortcuts without a manual.

**In-context.** From within a terminal, the palette biases toward session actions (broadcast, split, snippets to run here, disconnect) before global navigation.

**Why.** For 100+ hosts, search *is* the navigation; the sidebar/tree is for browsing, the palette is for going. Merging entity-jump and command-run into one surface means one muscle memory instead of two. Frecency ranking means the boxes you actually touch float up, so the palette gets faster the more you use it.

---

## 5. Live connection stepper

Shown when a connection is establishing — inline in the new tab's body (not a modal; it *is* the terminal-to-be).

**Structure.** A vertical `Progress`-driven stepper reflecting the host's *real* configuration and *real* events streamed from the Rust backend:

`Resolving host → TCP connect (host:port) → [Jump host: bastion] → Key exchange / algo negotiated → Host key verification → Authentication (id_ed25519 / password / agent) → [MFA: verification code] → Session open → Shell`

Only the steps that apply to *this* host appear — a host with no jump host and key auth shows a shorter chain than one chaining through a bastion with MFA. Each step shows a live status icon (spinner → check / ✗), and where useful, a detail line in mono (the negotiated cipher, the resolved IP, the key filename).

**Interactive steps.** When a step needs the user, it becomes a control *in place*:
- **Host key verification** — first connection or changed key: shows the fingerprint (mono, `SHA256:…`) with **Accept / Reject**; a *changed* key gets the loud red treatment (see §12).
- **Password / passphrase** — inline secure field.
- **MFA** — a 6-digit code input.

**On failure.** The failing step turns red with the actual error (auth failed, connection refused, timeout, host key mismatch) and offers the relevant recovery: **Retry**, **Edit host**, **Try password instead**, **Use different key**. No dead ends.

**On success.** The stepper collapses upward into a single-line "Connected in 320ms" toast-in-tab that fades, handing the full pane to xterm.js.

**Why.** This audience has been burned by clients that show a fake progress bar and then hang. A stepper wired to real events is a *trust* feature: when something's slow, you see exactly which stage — DNS? auth? bastion? — which is the difference between "the app is broken" and "the bastion is slow." It also turns MFA/host-key prompts from jarring modals into an expected step in a visible pipeline.

---

## 6. Terminal tab (single session)

The core workspace. Mostly it should get out of the way and be xterm.js at full bleed.

**Anatomy.** A near-invisible 24px pane header (host name + status dot + latency + a `⋯` for pane actions and a split handle), then the terminal filling everything else. The header auto-hides in fullscreen. Background respects the connection's theme (see §18), so you know *which* box you're on by its colors, not just the tab label.

**Find in terminal (`Cmd+F`).** A slim search bar drops from the top-right of the pane (not a modal): input + match count ("3/17") + up/down + case/regex toggles + close. It searches xterm.js scrollback, highlights matches, `Enter`/`Shift+Enter` cycle, `Esc` closes. Live-connection output keeps flowing underneath.

**Reconnect / auto-reconnect.** On a dropped connection the pane dims and shows a centered card: "Connection lost — reconnecting… (attempt 2)" with a countdown, a **Reconnect now** button, and **Stop**. Auto-reconnect uses backoff and is per-host configurable (off / N attempts / forever). On success the pane restores and a toast notes "Reconnected." Scrollback is preserved across the drop so context isn't lost. Mosh hosts get a "connection resumed" affordance rather than a full reconnect, reflecting Mosh's roaming.

**Pane actions (`⋯` / right-click):** split right/down, duplicate session, rename tab, set tab color, open SFTP for this host, start/stop logging, add to broadcast group, disconnect (confirm if live).

**Why.** The header is intentionally minimal — every pixel of chrome is a pixel not showing output. Theme-per-connection is a safety feature as much as an aesthetic one: color-coding prod red and staging blue is how you avoid running the wrong command on the wrong box. `Cmd+F` as a drop-down bar (not a modal) keeps you oriented in the scrollback while searching.

---

## 7. Split panes (drag-and-drop track — see splits.md)

Explicitly *not* the rejected 1/2/4 toolbar. Layout emerges from where you drop.

**Creating a split.** Drag a tab (or a host row, or a palette result via `Cmd+Enter`) into an existing pane. As you drag, the target pane shows **four drop zones** (left / right / top / bottom edges) plus a **center "replace/tabify" zone**; the hovered zone highlights with a `--primary` translucent overlay showing exactly where the new pane lands. Drop = split in that direction, each pane its own host/session. Nesting is unlimited (via recursive `Resizable` panel groups); dividers are draggable to resize.

**Per-pane control.** Every pane owns its header with its own close `×`. Closing one pane never ambiguously touches its siblings — the layout re-flows to fill the freed space. The **active pane** gets a 1px `--primary` border and its header brightens; focus routes keystrokes there. `Cmd+[` / `Cmd+]` or `Cmd+Opt+Arrow` move focus between panes.

**Confirm before killing sessions.** Any action that would drop live sessions — closing a pane, collapsing a layout, closing a split-tab — raises an `AlertDialog` that **names the count and the hosts**: "Close 2 panes? This disconnects **prod-db-01** and **cache-02**." Single dumb "are you sure" is avoided; the confirm states exactly what dies. Closing an *idle* (already-disconnected) pane skips the confirm.

**Saving a layout.** A useful arrangement (e.g. 4 web nodes) can be **saved as a Group** (§9) so it reopens in one click.

**Components:** `Resizable` (nested panel groups), a custom DnD layer (drop-zone overlays), `AlertDialog`, `ContextMenu`.

**Why.** The prototype failed because layout was chosen upfront and teardown was silent. Here, layout is a *consequence* of direct manipulation (drag where you want it), close is *per-pane and explicit*, and every session death is *named before it happens*. Those three exactly answer the three documented failures.

---

## 8. Terminal groups

A group is N hosts you open together, as one tab that contains horizontal sub-tabs (or a saved split layout).

**In the sidebar tree**, a group is a folder with a "play" affordance on hover — one click opens the whole group. Groups can nest (client → environment → box). 

**When opened**, a group becomes a single top-level tab whose body carries a **horizontal sub-tab strip** — one sub-tab per member host, each with its own status dot and accent. This keeps 6 related boxes under one top-level tab instead of scattering 6 tabs across the bar. Alternatively a group can store a **split layout** (from §7) and open as that arrangement.

**Managing.** Drag hosts into a group in the tree; reorder members; a group's `⋯` offers "Open all", "Open as split", "Broadcast to group" (§10), "Edit group" (name, icon, color, default open-mode).

**Why.** People think in projects, not individual machines. "Open my staging cluster" is the real unit of work; groups make that a single action and keep the tab bar sane. Nesting matches how this audience already organizes `~/.ssh/config` with comments and host patterns.

---

## 9. Broadcast input

Type once, send to many. A must-have for anyone running the same command across a fleet.

**Entering broadcast.** From a group or a multi-pane split: a **Broadcast** toggle in the toolbar / palette. When on, a persistent **broadcast bar** appears at the bottom of the active tab, clearly styled (amber-tinted, "Broadcasting to 4 sessions" with the host names as removable chips). Keystrokes typed there (or, optionally, directly in any member pane — a mode switch) fan out to all targets.

**Safety.** Broadcast is visually loud on purpose (persistent amber banner, every target pane gets an amber outline) because sending `rm` to 12 boxes at once is exactly the kind of foot-gun that needs constant, unmissable state. A **one-command mode** (send a single command then auto-exit broadcast) and an explicit **exclude** control (drop a pane from the fan-out) reduce blast radius. Turning broadcast off is one `Esc`.

**Why.** The danger of broadcast isn't sending — it's *forgetting it's on*. So the whole design optimizes for "you always know when it's live and to whom," rather than hiding it. Per-pane amber outlines mean even if you're deep in one terminal, you can't lose track that your keystrokes are multiplying.

---

## 10. Connection editor (add / edit host)

Where a host is configured. Reached from ＋New Host, `E` on a row, or "Edit host" anywhere. A `Dialog` (or full `Sheet` on smaller windows), organized so the 90% case is the top of one tab and the power features are behind other tabs — progressive disclosure.

**Tab: General (the default, everything most hosts need).**
- **Label** (display name), **Address** (host), **Port** (default 22, mono), **Username**.
- **Auth**: a `Select` — Key / Password / Agent — that reveals only the relevant control (key picker from Key Manager, or "prompt each time" for password).
- **Tags** (creatable multi-select), **Group/folder** (tree picker), **Icon** (auto/custom), **Favorite** toggle.
- A **Test connection** button that runs the §5 stepper in a preview strip without opening a full tab — instant validation that you got it right.

**Tab: Jump / Proxy.** ProxyJump chain builder (§11).

**Tab: Port forwarding.** Reusable forward rules for this host (§16), each a row: type (Local/Remote/Dynamic) + bind + target.

**Tab: Terminal & theme.** Per-connection theme, font, font-size, color-scheme override, per-tab accent color, and the initial command/startup snippet to run on connect.

**Tab: Advanced.** Keep-alive interval, compression, connection timeout, auto-reconnect policy, Mosh/Telnet toggle, session-logging toggle + path, environment variables, and the raw `ssh_config` options passthrough (an escape hatch — anything we don't model, you can still set, and it round-trips on export).

**Footer:** Cancel · Save · **Save & Connect** (primary).

**Why.** One flat form with 30 fields is how Termius-clones scare people. Tabbing it means a new user fills four fields on General and hits Save & Connect, while a power user still has every knob — just not all at once. "Test connection" inline closes the loop before the config leaves the dialog, which is faster than save→try→fail→reopen.

---

## 11. Jump host / ProxyJump chaining

Lives as a tab in the editor and as its own concept in the stepper.

**Builder.** An ordered, drag-reorderable list of hops: `you → [bastion-1] → [bastion-2] → target`. Each hop is a picker that can reference **an existing saved host** (preferred — reuse its auth/keys) or an inline host:port. Add-hop button between any two nodes. A live "path preview" renders the chain as a horizontal breadcrumb so a 2-bastion chain is legible at a glance.

**At connect time**, each bastion becomes its own step in the §5 stepper ("Jump host: bastion-1 ✓ → Jump host: bastion-2 ✓ → target"), so if hop 2 is where auth fails, you see it.

**Why.** Referencing saved hosts (not re-entering bastion creds) is the whole point — it means a bastion's key/MFA is configured once and reused across every host behind it. Surfacing each hop in the stepper turns the most confusing failure mode in SSH ("it hangs somewhere in the chain") into a pinpointed one.

---

## 12. Known-host fingerprint management

Two surfaces: the inline verification moment, and a management table.

**Inline (in the stepper).** First-ever connection: show `SHA256:…` fingerprint (mono, copyable), key type, and Accept/Reject. A **changed** fingerprint is a security event and looks like one — a red `AlertDialog`-weight panel: "⚠ Host key changed for prod-db-01. This could indicate a man-in-the-middle attack, or the server was legitimately rebuilt." It shows **old vs new** fingerprints side by side (mono, diff-highlighted), and refuses the easy default — the safe action (Reject/Cancel) is primary; accepting requires an explicit, deliberate click. No "remember" checkbox on a changed key.

**Management table** (under Settings → Security, and per-host): a `Table` of known hosts — host, key type, fingerprint (mono, truncated + copy), date added, source (imported / accepted here). Actions: revoke, re-verify, copy. Import from and export to `known_hosts`.

**Why.** A changed host key is the one moment where friction is *correct* — the design inverts the usual "make it easy to proceed" instinct and makes the safe choice the path of least resistance, because the cost of a mistaken accept here is a compromised session.

---

## 13. Key manager

Modeled on Termius: import existing keys and generate in-app. A sidebar section, rendered as a `Table`/list.

**Key list.** Each row: name, **type badge** (ed25519 / RSA-4096 / ECDSA / FIDO2, color-coded, mono), fingerprint (mono, copyable), where it's stored (**Keychain** badge — the trust anchor again), which hosts use it (count → click to see), created date. Row actions: copy public key, export public key, reveal in Keychain, delete (with a "used by N hosts" warning).

**Generate key (`Dialog`).** Type selector (ed25519 default + recommended, RSA with bit-size, ECDSA with curve, **FIDO2/hardware** — which prompts a touch on the security key), name, optional passphrase (strength meter), and a comment. On create, the public key is shown immediately with **Copy** and **Copy `ssh-copy-id` command** buttons — because the very next thing anyone does with a new key is get it onto a server.

**Import key.** Drag a private key file in, or paste; we detect type, validate, offer to store the secret in Keychain and keep only a reference. Passphrase-protected keys prompt once.

**Why.** The "Copy ssh-copy-id command" button is a small thing that removes a real, universal chore — it anticipates the next step. Showing the Keychain badge on every key reassures the security-minded user that nothing sensitive is sitting in a plist. FIDO2 gets first-class placement because hardware-key users are exactly the security-conscious segment this app courts.

---

## 14. Vault, Keychain & Touch ID

Security posture made visible, never nagging.

**Vault status pill** (bottom of sidebar, always visible): "Vault unlocked" (subtle) or "Locked" (amber) with a lock icon. Optional master-password mode: the app can require an unlock on launch/after idle. When locked, host *names* can still show (configurable) but secrets and connect are gated until unlock.

**Unlock.** A focused `Dialog`: a big Touch ID prompt ("Touch ID to unlock your vault") with a **Use master password** fallback. Touch ID is the default because it's the fastest honest gate. Auto-lock after configurable idle.

**Where secrets live.** Every secret (key passphrases, saved passwords) is in **macOS Keychain**; the app stores references. Settings → Security states this plainly and links each secret to its Keychain item. Nothing secret is ever written to the on-disk config (which stays `ssh_config`-shaped and shareable).

**Why.** This audience will *check*. Making "secrets are in Keychain, config is plaintext-safe" a visible, verifiable fact — rather than a marketing claim — is what earns trust. Touch-ID-first respects the speed principle: biometric unlock is sub-second, a master password is the fallback for when it's unavailable.

---

## 15. SFTP browser

Drag-and-drop file transfer, opened per-host (from a row action, pane `⋯`, or palette). Opens as its own tab or a side-by-side pane against the terminal.

**Layout.** A **dual-pane file manager**: local filesystem on the left, remote on the right (or a single remote pane with your Finder as the drag source). Each pane: breadcrumb path (mono, editable), a `Table` of entries (name, size, modified, permissions in mono `rwxr-xr-x`), sort, and a filter field. Hidden-files toggle.

**Transfer.** Drag files/folders between panes (or from Finder) to transfer. Each transfer appears in a **transfer tray** (`Sheet` docked bottom): per-item progress, speed, ETA, pause/cancel, and a clear-completed. Conflicts prompt (overwrite / skip / rename / apply-to-all). Directory transfers recurse with an aggregate progress.

**Editing.** Double-click a text file to open it in a quick editor (or hand off to `$EDITOR` locally), edit, save-back over SFTP — the common "tweak one config file" loop without a manual download/upload dance.

**Why.** Dual-pane is the muscle memory transfer users already have (from every FTP client ever). The transfer tray as a non-blocking docked sheet means you can keep working in the terminal while a big transfer runs — reinforcing that this is one app, not a modal file dialog bolted on.

---

## 16. Snippets

Saved commands/scripts, runnable in one click across sessions. Sidebar section.

**Snippet list.** Grouped by folder, each: name, a mono preview of the command, tags. Row actions: run in active session, run in… (pick target/broadcast), copy, edit.

**Editor.** Name, the command/script body (mono, multi-line, syntax-lit), and — answering an open question in the requirements — **parameterized variables**: tokens like `{{service}}` or `{{branch}}` are detected and, when you run the snippet, a tiny inline form prompts for each value before sending. Optional per-snippet: run-on-connect association with hosts, and "confirm before running" for dangerous ones.

**Running.** From the palette ("run snippet…"), a pane's `⋯`, or the snippet list. With broadcast on, a snippet fans out to the group. Parameterized snippets show their fill-in form first.

**Why.** Plain strings cover the basics but the recurring real need is "the same command with today's value" (`kubectl logs {{pod}}`, `git checkout {{branch}}`) — parameterization is what makes snippets a genuine time-saver rather than a clipboard with extra steps. Keeping "confirm before running" per-snippet respects that some saved commands are routine and some are loaded guns.

---

## 17. Port forwarding

Reusable forward configs — local, remote, dynamic/SOCKS. A sidebar section *and* a per-host editor tab (§10).

**Forwards table.** Each rule: type (`Local` / `Remote` / `Dynamic` badge), a readable route (`localhost:8080 → prod-db:5432` for local; `:9000 ← remote` for remote; `SOCKS :1080` for dynamic), the host it rides on, and a **status toggle** (`Switch`) with a live state dot. Toggling activates/deactivates the tunnel immediately; a live tunnel shows throughput.

**Add/edit forward.** A small form that adapts to type — Local asks bind-port + destination host:port; Remote flips the direction; Dynamic asks only a local SOCKS port. Plain-language helper text under each ("Local: reach a remote service as if it were on your machine") because forward direction is perennially confusing.

**Why.** Making forwards *saved, named, and toggleable* — instead of ssh flags you retype — matches how people actually use them (the same 8080→db tunnel every day). The adaptive form + plain-language hints fight the single biggest usability problem with port forwarding: nobody remembers which way `-L` vs `-R` points.

---

## 18. Theming

**Global theme:** dark (default) / light / system, plus a set of built-in terminal color schemes (Solarized, Dracula, Nord, etc.) selectable app-wide.

**Per-connection theme:** set at config time (§10 → Terminal & theme) — color scheme, font, font size. This is the "prod is red, staging is amber, local is green" safety mechanism: the terminal background itself tells you where you are.

**Per-tab accent color:** the 8-preset (+custom) rail shown on tabs and host rows. Distinct from the terminal color scheme — accent is for *identification*, the scheme is for *the reading surface*.

**Theme editor** (Settings → Appearance): pick a scheme, live-preview against a sample terminal, tweak the 16 ANSI colors + bg/fg/cursor, save as a custom scheme, export/import scheme JSON.

**Why.** Two separate color concerns are deliberately kept separate: a *tab accent* (small, decorative-by-permission, for telling tabs apart) and a *terminal scheme* (the whole surface, semantic — which environment am I in). Conflating them is how people end up with a pretty-but-useless rainbow; splitting them lets color carry meaning.

---

## 19. Import / export (no lock-in)

The "you're never trapped" guarantee, made concrete.

**Import (`Dialog`, from first-run empty state or Settings):**
- **`~/.ssh/config`** — auto-detected; parses Host blocks, HostName, User, Port, IdentityFile, ProxyJump, and common options into saved hosts. Shows a **preview table** of what will be created (with detected group-by-comment), lets you deselect any, and maps `IdentityFile`s to Key Manager entries.
- **PuTTY sessions** (`.reg` / registry) — for migrants from Windows/PuTTY.
- **`known_hosts`** — into the fingerprint store (§12).

**Export:**
- **Standard `ssh_config`** — every saved host round-trips to a valid `ssh_config`, including the raw-options passthrough (§10 Advanced) so nothing is lost. This is the anti-lock-in promise: what you import, you can always get back out.
- Export saved connections/config bundle (for backup/manual move).

**Why.** Import-from-`ssh_config` is the make-or-break onboarding path for this audience — a client that can't read the config they already maintain is dead on arrival. The preview-and-deselect step respects that their config may have junk they don't want imported. Clean round-trip export is the trust close: they'll try it early precisely to check they're not locked in, so it has to be flawless.

---

## 20. History

A chronological record of connections. Sidebar section, rendered as a grouped `Table`.

Entries: host, when, duration, outcome (connected / failed / dropped — the status language), and the auth method used. Grouped by day. Filterable and searchable. Actions: reconnect (one click), "create host from this" (for one-off ad-hoc connections you now want to save), copy address. Feeds the frecency ranking in the palette and the "Recent" cluster on the host list.

**Why.** History does double duty: it's an audit trail (what did I connect to, when, did it fail) *and* a fast reconnect surface. "Create host from this" captures the common pattern of connecting ad-hoc once, then deciding to keep it — without retyping.

---

## 21. Settings

A standard `Dialog`/page with a left nav: **General** (startup behavior, default shell, density default, telemetry), **Appearance** (themes, schemes, editor from §18), **Security** (vault/master-password, auto-lock, Touch ID, Keychain overview, known-hosts table), **Keys** (defaults for generation), **Connectivity** (default keep-alive, reconnect policy, Mosh/Telnet), **Import/Export**, **Shortcuts** (full remappable keymap — power users will remap), **About**.

**Telemetry** (answering the open question): a single clear toggle, **opt-in and off by default**, with a plain-language "here's exactly what's collected" expandable list (app version, crash traces, coarse feature counts — never host names, addresses, or any content). One-click "view raw payload" so the skeptical can verify. This honesty *is* the local-first brand.

**Why.** Making telemetry opt-in/off-by-default with a visible payload is the only choice consistent with the local-first, no-lock-in promise; anything else undermines the whole positioning for exactly the users most likely to notice.

---

## 22. Cross-cutting: keyboard map (starter)

Learnability comes from consistency; here's the spine (all remappable in Settings → Shortcuts):

`Cmd+K` palette · `Cmd+P` host jump · `Cmd+T` new tab · `Cmd+W` close tab (confirm if live) · `Cmd+F` find in terminal · `/` focus host search · `Cmd+Enter` connect in split · `Cmd+D` split right · `Cmd+Shift+D` split down · `Cmd+[ / ]` focus prev/next pane · `Cmd+1…9` jump to tab · `Cmd+B` toggle broadcast · `Cmd+L` lock vault · `Cmd+R` reconnect · `Esc` cancel/close overlay.

---

## 23. Open questions carried from requirements (design's answers)

1. **Telemetry** — opt-in, off by default, visible payload (§21).
2. **Snippets** — parameterized with `{{variables}}`, not plain strings only (§16).
3. **Session logging** — per-host toggle in editor Advanced; default path `~/…/logs/<host>/<date>.log`, size-based rotation, surfaced in the host row's `⋯` (§6, §10).
4. **Mosh** — kept in v1 but the editor's Mosh toggle shows an inline note that it requires the `mosh-server` binary on the host, and the stepper reports clearly if it's missing (so the server-side dependency is a visible, explained requirement, not a silent failure).

---

## 24. What to build first (design priority, mirrors the phased MVP)

The screens that must be flawless for MVP: **host list (§3), command palette (§4), connection stepper (§5), terminal tab + find + reconnect (§6), key manager (§13), vault/Touch ID (§14), jump hosts (§11), known-hosts (§12), theming (§18), import/export (§19).** Splits (§7), SFTP (§15), snippets (§16), port forwarding (§17), broadcast (§9), and groups-as-splits are the fast-follow/dedicated tracks — designed here so the shell accommodates them, but not gating the first ship.
