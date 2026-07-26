//! Secret storage in the OS keychain (macOS Keychain / Windows Credential
//! Manager via `keyring`). Secrets — passwords and key passphrases — live here
//! only, keyed by host id. Nothing secret is written to `hosts.json` or logs.
//! Touch ID gating and the vault UI (design-spec §14) are a later pass.

use keyring::Entry;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const SERVICE: &str = "com.ssh-client.secrets";

// Process-lifetime cache of already-read secrets. macOS prompts for Keychain
// access on every confidential *read* unless the app is a stably code-signed,
// ACL-trusted binary (dev builds are re-signed each run, so "Always Allow"
// never sticks and every read re-prompts). One connect reads the same secret
// more than once (existence probe + actual use, plus reconnects), which is why
// the prompt appeared several times. Memoizing collapses those to a single
// prompt per secret per app run. Secrets already live in memory during a
// session (russh holds the key/password), so this doesn't widen exposure.
// ponytail: never evicted; fine for a desktop app's handful of secrets.
fn cache() -> &'static Mutex<HashMap<String, Option<String>>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn entry(host_id: &str) -> Result<Entry, String> {
    Entry::new(SERVICE, host_id).map_err(|e| format!("keychain: {e}"))
}

/// Backend-internal read — deliberately NOT a command, so the webview can never
/// pull a plaintext secret back out. Cached after the first Keychain read.
pub fn get(host_id: &str) -> Option<String> {
    if let Some(hit) = cache().lock().unwrap().get(host_id) {
        return hit.clone();
    }
    let value = entry(host_id).ok().and_then(|e| e.get_password().ok());
    cache().lock().unwrap().insert(host_id.to_string(), value.clone());
    value
}

/// Best-effort delete used by host cleanup; a missing entry is not an error.
pub fn delete(host_id: &str) {
    if let Ok(e) = entry(host_id) {
        let _ = e.delete_credential();
    }
    cache().lock().unwrap().remove(host_id);
}

#[tauri::command]
pub fn secret_set(host_id: String, secret: String) -> Result<(), String> {
    entry(&host_id)?
        .set_password(&secret)
        .map_err(|e| format!("keychain set: {e}"))?;
    cache().lock().unwrap().insert(host_id, Some(secret));
    Ok(())
}

#[tauri::command]
pub fn secret_has(host_id: String) -> bool {
    get(&host_id).is_some()
}

#[tauri::command]
pub fn secret_delete(host_id: String) -> Result<(), String> {
    cache().lock().unwrap().remove(&host_id);
    match entry(&host_id)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("keychain delete: {e}")),
    }
}
