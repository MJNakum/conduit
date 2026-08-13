# Conduit

A modern, local-first SSH client for macOS, Windows, and Linux. Fast startup, low
memory, no account required. Inspired by mtPuTTY's multi-tab session management and
Termius's modern feature set.

Built with [Tauri](https://tauri.app) (Rust backend) + Svelte 5 + xterm.js. SSH and
PTY run in Rust via [`russh`](https://github.com/Eugeny/russh); the webview is
presentation only.

## Install

### Linux (Debian / Ubuntu) — apt

One-time setup adds Conduit's signed apt repository, then it updates like any package:

```sh
curl -fsSL https://mjnakum.github.io/conduit-apt/pubkey.asc \
  | sudo gpg --dearmor -o /usr/share/keyrings/conduit.gpg
echo "deb [signed-by=/usr/share/keyrings/conduit.gpg] https://mjnakum.github.io/conduit-apt stable main" \
  | sudo tee /etc/apt/sources.list.d/conduit.list
sudo apt update && sudo apt install conduit
```

After that, new versions arrive with `sudo apt upgrade`. (Currently amd64 only.)
Prefer a single file? Grab the `.AppImage` or `.deb` from
[Releases](https://github.com/MJNakum/conduit/releases/latest).

**Secret storage.** Conduit keeps passwords and private keys in your desktop's
Secret Service — GNOME Keyring, KWallet, or KeePassXC — which the `.deb`
recommends. Sessions without one (a minimal window manager, a headless box)
get an encrypted file instead, unlocked by a passphrase you set once. Settings
→ Secret storage shows which is in use and how to switch.

### macOS / Windows

Get the latest from the **[download page](https://mjnakum.github.io/conduit/)** or
**[GitHub Releases](https://github.com/MJNakum/conduit/releases/latest)**:

- **macOS** — `.dmg` (universal: Apple Silicon + Intel)
- **Windows** — `-setup.exe` (NSIS) or `.msi`

The app updates itself: it checks for new releases on launch, and you can trigger a
check from **Settings → About**.

#### Installing an unsigned build

macOS/Windows builds are currently unsigned, so the OS shows a first-run warning:

- **Windows** — on the SmartScreen prompt, click *More info* → *Run anyway*.
- **macOS** — right-click the app and choose *Open*, or run
  `xattr -dr com.apple.quarantine /Applications/Conduit.app`.

## Features

Local-first host management, tabbed terminals, live connection stepper, key manager
(ed25519/RSA/ECDSA) with OS-keychain storage and biometric vault lock (Touch ID /
Windows Hello), ProxyJump chaining, known-host management, SFTP, port forwarding,
snippets, broadcast input, session logging, `ssh_config` import/export, theming, and
full keyboard operability.

## Development

Requires Rust, Node, and [pnpm](https://pnpm.io).

```sh
pnpm install
pnpm tauri dev      # run the app (Vite on :1420 + the Rust shell)
pnpm tauri build    # production bundle
pnpm check          # Svelte/TS type-check
cargo test          # Rust tests (run in src-tauri/)
```

## Releasing

1. Bump `version` in `package.json`, `src-tauri/tauri.conf.json`, and
   `src-tauri/Cargo.toml`.
2. Push a `v*` tag (e.g. `git tag v0.2.0 && git push origin v0.2.0`).
3. The **Release** workflow builds macOS + Windows + Linux bundles, signs the updater
   artifacts, and creates a **draft** GitHub Release with `latest.json`.
4. Review and **publish** the draft — the download page and in-app updater read the
   latest *published* release.
5. Run the **Publish APT** workflow in
   [`conduit-apt`](https://github.com/MJNakum/conduit-apt) (or let it fire
   automatically) to push the new `.deb` to the apt repository.

The updater signing keypair is generated with `pnpm tauri signer generate`; the
private key and its password live in the `TAURI_SIGNING_PRIVATE_KEY` /
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` repository secrets, and the public key is in
`src-tauri/tauri.conf.json`.
