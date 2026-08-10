# ssh-client

A modern, local-first SSH client for macOS and Windows. Fast startup, low memory,
no account required. Inspired by mtPuTTY's multi-tab session management and Termius's
modern feature set.

Built with [Tauri](https://tauri.app) (Rust backend) + Svelte 5 + xterm.js. SSH and
PTY run in Rust via [`russh`](https://github.com/Eugeny/russh); the webview is
presentation only.

## Download

Get the latest build from the **[download page](https://mjnakum.github.io/ssh-client/)**,
or grab an installer directly from
**[GitHub Releases](https://github.com/MJNakum/ssh-client/releases/latest)**:

- **macOS** — `.dmg` (universal: Apple Silicon + Intel)
- **Windows** — `-setup.exe` (NSIS) or `.msi`

The app updates itself: it checks for new releases on launch, and you can trigger a
check from **Settings → About**.

### Installing an unsigned build

Builds are currently unsigned, so the OS shows a first-run warning:

- **Windows** — on the SmartScreen prompt, click *More info* → *Run anyway*.
- **macOS** — right-click the app and choose *Open*, or run
  `xattr -dr com.apple.quarantine /Applications/ssh-client.app`.

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

1. Bump `version` in `package.json` and `src-tauri/tauri.conf.json`.
2. Push a `v*` tag (e.g. `git tag v0.1.1 && git push origin v0.1.1`).
3. The **Release** workflow builds macOS + Windows bundles, signs the updater
   artifacts, and creates a **draft** GitHub Release with `latest.json`.
4. Review and **publish** the draft — the download page and in-app updater read the
   latest *published* release.

The updater signing keypair is generated with `pnpm tauri signer generate`; the
private key and its password live in the `TAURI_SIGNING_PRIVATE_KEY` /
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` repository secrets, and the public key is in
`src-tauri/tauri.conf.json`.
