// In-app auto-update. Wraps the Tauri updater plugin: check the GitHub Releases
// endpoint (see tauri.conf.json plugins.updater), and on an available update
// download + install it and relaunch into the new version. The signature is
// verified by the plugin against the bundled minisign public key.
import { check } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import { toast } from './toast.svelte'

export const updateState = $state({ checking: false })

// `manual` distinguishes the user pressing "Check for updates" (report the
// no-update and error cases) from the silent startup check (stay quiet unless
// there's actually something to install).
export async function checkForUpdates(manual = false) {
  if (updateState.checking) return
  updateState.checking = true
  try {
    const update = await check()
    if (!update) {
      if (manual) toast('You are on the latest version')
      return
    }
    toast(`Updating to v${update.version}…`)
    await update.downloadAndInstall()
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
