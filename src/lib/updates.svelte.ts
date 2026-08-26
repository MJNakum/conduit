// In-app auto-update. Wraps the Tauri updater plugin: check the GitHub Releases
// endpoint (see tauri.conf.json plugins.updater), and on an available update
// download + install it and relaunch into the new version. The signature is
// verified by the plugin against the bundled minisign public key.
//
// Package-managed installs (.deb, .rpm) are the exception: they check, but they
// never install. See MANAGED below.
import { invoke } from '@tauri-apps/api/core'
import { check } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import { toast } from './toast.svelte'

// `available` is set only for package-managed installs, where we report the new
// version and the command instead of installing it ourselves.
export const updateState = $state({
  checking: false,
  available: null as { version: string; command: string } | null,
})

// tauri-plugin-updater defaults to `timeout: None` (2.10.1 updater.rs:176), so a
// stalled connection to GitHub never settles — no resolve, no reject. That
// wedged the whole feature: the silent startup check would hang, `checking`
// stayed true for the life of the process, and Settings > About showed
// "Checking…" forever with the button disabled. Even enabled it would have done
// nothing, because the re-entrancy guard turns every later press into a no-op
// while a check is in flight.
const CHECK_MS = 20_000
// Generous: covers downloading a ~85MB AppImage on a slow link.
const DOWNLOAD_MS = 10 * 60_000

// Installs owned by a package manager. Updating these in-process would run
// `dpkg -i` behind pkexec — which needs a polkit agent a minimal desktop may not
// have — and would drop a package underneath the manager that owns the app. The
// package manager is the update path; we only say so.
const MANAGED: Record<string, string> = {
  deb: 'sudo apt update && sudo apt upgrade conduit',
  rpm: 'sudo dnf upgrade conduit',
}

let installKind: string | null = null
async function kindOfInstall(): Promise<string> {
  // Patched into the binary at bundle time, so it cannot change while running.
  installKind ??= await invoke<string>('install_kind').catch(() => 'unknown')
  return installKind
}

// The plugin's timeout covers its HTTP request; this covers the whole call, so a
// hang anywhere — IPC, the install helper — still releases the button.
function withDeadline<T>(work: Promise<T>, ms: number, what: string): Promise<T> {
  let timer: ReturnType<typeof setTimeout>
  const deadline = new Promise<never>((_, reject) => {
    timer = setTimeout(() => reject(new Error(`${what} timed out after ${Math.round(ms / 1000)}s`)), ms)
  })
  return Promise.race([work, deadline]).finally(() => clearTimeout(timer)) as Promise<T>
}

// `manual` distinguishes the user pressing "Check for updates" (report the
// no-update and error cases) from the silent startup check (stay quiet unless
// there's actually something to install).
export async function checkForUpdates(manual = false) {
  if (updateState.checking) return
  updateState.checking = true
  try {
    const update = await withDeadline(check({ timeout: CHECK_MS }), CHECK_MS + 5_000, 'Update check')
    if (!update) {
      updateState.available = null
      if (manual) toast('You are on the latest version')
      return
    }

    const command = MANAGED[await kindOfInstall()]
    if (command) {
      // Surfaced in Settings > About with the command to copy, so this is worth
      // saying even on the silent startup check — it is news, not a nag.
      updateState.available = { version: update.version, command }
      toast(`v${update.version} is available — update with your package manager`)
      return
    }

    toast(`Updating to v${update.version}…`)
    await withDeadline(
      update.downloadAndInstall(undefined, { timeout: DOWNLOAD_MS }),
      DOWNLOAD_MS,
      'Update download',
    )
    await relaunch()
  } catch (e) {
    // A silent startup check must not nag on a network hiccup or a dev build
    // with no release endpoint; only a manual check surfaces the failure.
    if (manual) toast(`Update check failed: ${e}`, 'err')
    console.error('update check failed', e)
  } finally {
    updateState.checking = false
  }
}
