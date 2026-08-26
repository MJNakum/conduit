// In-app auto-update. Wraps the Tauri updater plugin: check the GitHub Releases
// endpoint (see tauri.conf.json plugins.updater), and on an available update
// download + install it and relaunch into the new version. The signature is
// verified by the plugin against the bundled minisign public key.
import { check } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import { toast } from './toast.svelte'

export const updateState = $state({ checking: false })

// tauri-plugin-updater defaults to `timeout: None`, so a stalled connection to
// GitHub never settles — no resolve, no reject. That wedged the whole feature:
// the silent startup check would hang, `checking` stayed true for the life of
// the process, and Settings > About showed "Checking…" forever with the button
// disabled. Even enabled it would have done nothing, because the re-entrancy
// guard below turns every later press into a no-op while a check is in flight.
const CHECK_MS = 20_000
// Generous: this covers downloading a ~85MB AppImage on a slow link, and on a
// .deb it also spans the pkexec password prompt, which waits on the user.
const DOWNLOAD_MS = 10 * 60_000

// The plugin's timeout covers its HTTP request; this covers the whole call, so
// a hang anywhere — IPC, the install helper — still releases the button.
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
      if (manual) toast('You are on the latest version')
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
