// Which store is holding passwords and private keys, and the passphrase flow
// for the Linux encrypted-file fallback. Presentation only — plaintext secrets
// never cross into the webview; these commands move a passphrase in and a
// status back out.
//
// CAUTION: `loadStatus()` forces the backend to probe for a Secret Service. On a
// machine whose keyring is installed but locked, that probe can raise the
// desktop's own unlock dialog. Call it when the user opens the Keys page or
// Settings > Secret storage — deliberate visits to credential UI, where a prompt
// is expected — and after a secret operation fails. Never from App's onMount:
// that would put a D-Bus round trip, and possibly a modal, on the launch path.
import { invoke } from '@tauri-apps/api/core'
import { passphraseDialog } from './dialog.svelte'

export type SecretStatus = {
  kind: 'keyring' | 'file'
  label: string
  detail: string
  locked: boolean
  uninitialized: boolean
  store_file_exists: boolean
  pinned: 'file' | 'keyring' | null
  linux: boolean
}

export const secretsState = $state<{ status: SecretStatus | null; busy: boolean }>({
  status: null,
  busy: false,
})

export async function loadStatus(): Promise<SecretStatus | null> {
  try {
    secretsState.status = await invoke<SecretStatus>('secret_backend_status')
  } catch {
    secretsState.status = null
  }
  return secretsState.status
}

/** Short name for badges and toasts; falls back to something true-everywhere. */
export function storeName(): string {
  return secretsState.status?.label ?? 'secret storage'
}

export async function pinBackend(kind: 'file' | 'keyring' | null) {
  await invoke('secret_backend_pin', { kind })
  await loadStatus()
}

/**
 * Make sure secrets are writable, prompting only when there is something the
 * user must actually decide. Returns false if they cancelled or it failed —
 * callers should abandon the operation rather than report a false success.
 */
export async function ensureUsable(): Promise<boolean> {
  const s = secretsState.status ?? (await loadStatus())
  if (!s || s.kind !== 'file') return true
  if (!s.locked && !s.uninitialized) return true

  if (s.uninitialized) {
    const pass = await passphraseDialog({
      title: 'Set a passphrase for secret storage',
      message:
        'This session has no system keyring, so Conduit will keep passwords and private keys in an encrypted file.\n\n' +
        'There is no recovery. If you forget this passphrase, everything stored here is lost and you will need to add it again.',
      okLabel: 'Set passphrase',
      confirm: true,
    })
    if (pass === null) return false
    secretsState.busy = true
    try {
      secretsState.status = await invoke<SecretStatus>('secret_store_create', { passphrase: pass })
      return true
    } finally {
      secretsState.busy = false
    }
  }

  // Locked. A wrong passphrase re-prompts with the reason rather than dropping
  // whatever the user was in the middle of doing.
  let why = 'Enter the passphrase for your encrypted secret store.'
  for (;;) {
    const pass = await passphraseDialog({ title: 'Unlock secret storage', message: why, okLabel: 'Unlock' })
    if (pass === null) return false
    secretsState.busy = true
    try {
      secretsState.status = await invoke<SecretStatus>('secret_store_unlock', { passphrase: pass })
      return true
    } catch (e) {
      why = String(e)
    } finally {
      secretsState.busy = false
    }
  }
}
