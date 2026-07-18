//! Secret storage in the OS keychain (macOS Keychain / Windows Credential
//! Manager via `keyring`). Secrets — passwords and key passphrases — live here
//! only, keyed by host id. Nothing secret is written to `hosts.json` or logs.
//! Touch ID gating and the vault UI (design-spec §14) are a later pass.

use keyring::Entry;

const SERVICE: &str = "com.ssh-client.secrets";

fn entry(host_id: &str) -> Result<Entry, String> {
    Entry::new(SERVICE, host_id).map_err(|e| format!("keychain: {e}"))
}

/// Backend-internal read — deliberately NOT a command, so the webview can never
/// pull a plaintext secret back out.
pub fn get(host_id: &str) -> Option<String> {
    entry(host_id).ok()?.get_password().ok()
}

/// Best-effort delete used by host cleanup; a missing entry is not an error.
pub fn delete(host_id: &str) {
    if let Ok(e) = entry(host_id) {
        let _ = e.delete_credential();
    }
}

#[tauri::command]
pub fn secret_set(host_id: String, secret: String) -> Result<(), String> {
    entry(&host_id)?
        .set_password(&secret)
        .map_err(|e| format!("keychain set: {e}"))
}

#[tauri::command]
pub fn secret_has(host_id: String) -> bool {
    get(&host_id).is_some()
}

#[tauri::command]
pub fn secret_delete(host_id: String) -> Result<(), String> {
    match entry(&host_id)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("keychain delete: {e}")),
    }
}
