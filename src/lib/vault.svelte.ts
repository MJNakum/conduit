// Vault lock state. Locking hides the app behind a gate; unlocking requires
// device-owner auth (Touch ID / macOS password) via the Rust `vault_authenticate`
// command. No secret is read here — this only gates the UI.
import { invoke } from '@tauri-apps/api/core'

export const vault = $state({ locked: false, authing: false })

export function lockVault() {
  vault.locked = true
}

export async function unlockVault(): Promise<boolean> {
  if (vault.authing) return false
  vault.authing = true
  try {
    const ok = await invoke<boolean>('vault_authenticate', { reason: 'Unlock your SSH vault' })
    if (ok) vault.locked = false
    return ok
  } catch {
    return false
  } finally {
    vault.authing = false
  }
}
