//! Secret storage — passwords and private keys, keyed by host id (or `key:<id>`
//! for managed keys). Nothing secret is written to `hosts.json`, `keys.json`, or
//! logs.
//!
//! Two backends:
//!
//! - **System keyring** (`keyring` crate): macOS Keychain, Windows Credential
//!   Manager, and on Linux the D-Bus Secret Service (GNOME Keyring / KWallet /
//!   KeePassXC). Always preferred — it is already unlocked at login and users
//!   trust it.
//! - **Encrypted file store** (`filestore.rs`, Linux only): the fallback for
//!   sessions with no Secret Service — minimal desktops, tiling WMs, headless
//!   boxes. Guarded by a passphrase the user sets once.
//!
//! The backend is probed once, lazily, on the first secret access. The frontend
//! triggers that shortly after first paint so it can ask for the file store's
//! passphrase up front instead of letting the first connection fail — the probe
//! is off the render path, not on it. On macOS and Windows the choice is a
//! compile-time constant with no runtime probe at all.

use keyring::Entry;
use serde::Serialize;
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

// ---------------------------------------------------------------------------
// Data directory
// ---------------------------------------------------------------------------

// `get`/`delete` are called from the SSH and host layers, which hold no
// AppHandle, but the file store needs the app data dir. Resolve it once from
// the Tauri setup hook (see `lib.rs`) instead of threading AppHandle through
// every call site.
static DATA_DIR: OnceLock<std::path::PathBuf> = OnceLock::new();

pub fn init_data_dir(dir: std::path::PathBuf) {
    let _ = DATA_DIR.set(dir);
}

#[cfg(target_os = "linux")]
fn data_dir() -> Result<&'static std::path::Path, String> {
    DATA_DIR
        .get()
        .map(|p| p.as_path())
        .ok_or_else(|| "app data directory is not available".to_string())
}

// ---------------------------------------------------------------------------
// Backend selection
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Keyring,
    /// Linux with no Secret Service on the session bus.
    #[cfg(target_os = "linux")]
    File,
}

/// Account used only to ask the keyring whether it is answering at all. Reading
/// a nonexistent entry is the cheapest probe that still round-trips to the
/// daemon; it never creates anything.
#[cfg(target_os = "linux")]
const PROBE_ACCOUNT: &str = "__conduit_backend_probe";

#[cfg(not(target_os = "linux"))]
fn kind() -> Kind {
    Kind::Keyring
}

/// Sits next to `keys.json`. Written only when the user explicitly picks a
/// backend in the UI; absent means "decide automatically".
#[cfg(target_os = "linux")]
const OVERRIDE_FILE: &str = "secrets.backend";

#[cfg(target_os = "linux")]
fn kind() -> Kind {
    static KIND: OnceLock<Kind> = OnceLock::new();
    *KIND.get_or_init(detect)
}

#[cfg(target_os = "linux")]
fn detect() -> Kind {
    // 1. Explicit choice always wins — env first (handy for testing and for
    //    exotic setups), then the persisted Settings choice.
    match std::env::var("CONDUIT_SECRET_BACKEND").as_deref() {
        Ok("file") => return Kind::File,
        Ok("keyring") => return Kind::Keyring,
        _ => {}
    }
    if let Ok(dir) = data_dir() {
        match std::fs::read_to_string(dir.join(OVERRIDE_FILE))
            .map(|s| s.trim().to_string())
            .as_deref()
        {
            Ok("file") => return Kind::File,
            Ok("keyring") => return Kind::Keyring,
            _ => {}
        }
    }

    // 2. No session bus at all — a headless box, an SSH session, a bare tty.
    //    Two syscalls, and it avoids a D-Bus connect that can block rather than
    //    fail fast when DBUS_SESSION_BUS_ADDRESS points somewhere stale.
    let has_bus = std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some()
        || std::env::var_os("XDG_RUNTIME_DIR")
            .map(|d| std::path::Path::new(&d).join("bus").exists())
            .unwrap_or(false);
    if !has_bus {
        return Kind::File;
    }

    // 3. Ask the service. Reading a nonexistent entry round-trips to the daemon
    //    and opens the collection without ever creating anything.
    match Entry::new(SERVICE, PROBE_ACCOUNT).and_then(|e| e.get_password()) {
        // A live service answering "no such entry" is the expected success.
        Ok(_) | Err(keyring::Error::NoEntry) => Kind::Keyring,
        Err(_) => Kind::File,
    }
}

/// Pin the backend for future launches, or clear the pin with `None`. Lets a
/// user who built up secrets in the file store keep reaching them after a
/// Secret Service appears on the box (and the reverse).
#[tauri::command]
pub fn secret_backend_pin(kind: Option<String>) -> Result<(), String> {
    #[cfg(not(target_os = "linux"))]
    {
        let _ = kind;
        Err("this platform only has the system keyring".into())
    }

    #[cfg(target_os = "linux")]
    {
        let path = data_dir()?.join(OVERRIDE_FILE);
        match kind.as_deref() {
            None => {
                let _ = std::fs::remove_file(&path);
                Ok(())
            }
            Some(k @ ("file" | "keyring")) => {
                std::fs::create_dir_all(data_dir()?).map_err(|e| format!("create data dir: {e}"))?;
                std::fs::write(&path, k).map_err(|e| format!("save backend choice: {e}"))
            }
            Some(_) => Err("unknown backend".into()),
        }
    }
}

#[cfg(target_os = "linux")]
fn store() -> &'static Mutex<Option<crate::filestore::FileStore>> {
    static STORE: OnceLock<Mutex<Option<crate::filestore::FileStore>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(None))
}

fn entry(account: &str) -> Result<Entry, String> {
    Entry::new(SERVICE, account).map_err(|e| format!("keychain: {}", describe(&e)))
}

/// Turn a `keyring::Error` into something a person can act on. The raw Display
/// for the Linux failure is `Platform secure storage failure: DBus error: The
/// name org.freedesktop.secrets was not provided by any .service files`, which
/// tells a user nothing — it used to be shown verbatim on the Keys page.
fn describe(e: &keyring::Error) -> String {
    match e {
        keyring::Error::NoEntry => "no secret stored for this entry".into(),
        keyring::Error::Ambiguous(_) => "more than one stored secret matches this entry".into(),
        keyring::Error::TooLong(what, max) => format!("{what} is longer than the {max} the platform allows"),
        keyring::Error::Invalid(what, why) => format!("{what} is not usable: {why}"),
        keyring::Error::NoStorageAccess(_) => {
            "the system keyring refused access — it may be locked".into()
        }
        keyring::Error::PlatformFailure(inner) => {
            let text = inner.to_string();
            if text.contains("org.freedesktop.secrets") || text.contains("ServiceUnknown") {
                "no secret storage service is running in this desktop session".into()
            } else {
                format!("the system keyring is not working: {text}")
            }
        }
        other => format!("the system keyring is not working: {other}"),
    }
}

// ---------------------------------------------------------------------------
// Status, reported to the UI
// ---------------------------------------------------------------------------

#[derive(Serialize, Clone)]
pub struct BackendStatus {
    /// "keyring" | "file"
    pub kind: String,
    /// Short name for badges and toasts, e.g. "System keyring".
    pub label: String,
    /// One sentence explaining the current state.
    pub detail: String,
    /// File store exists on disk but has not been unlocked this run.
    pub locked: bool,
    /// File store has never been set up — the user must choose a passphrase.
    pub uninitialized: bool,
    /// An encrypted file store exists on disk, whether or not it is in use. Lets
    /// Settings offer "use the encrypted file instead" to someone whose secrets
    /// are stranded there after a Secret Service appeared on the machine.
    pub store_file_exists: bool,
    /// The user pinned this backend explicitly: "file" | "keyring" | null.
    pub pinned: Option<String>,
    /// True when this platform can host a D-Bus Secret Service, i.e. when the
    /// setup guidance is worth showing.
    pub linux: bool,
}

#[tauri::command]
pub fn secret_backend_status() -> BackendStatus {
    #[cfg(not(target_os = "linux"))]
    {
        let label = if cfg!(target_os = "macos") {
            "macOS Keychain"
        } else {
            "Windows Credential Manager"
        };
        BackendStatus {
            kind: "keyring".into(),
            label: label.into(),
            detail: format!("Secrets are stored in the {label}."),
            locked: false,
            uninitialized: false,
            store_file_exists: false,
            pinned: None,
            linux: false,
        }
    }

    #[cfg(target_os = "linux")]
    {
        let exists = data_dir().map(crate::filestore::exists).unwrap_or(false);
        let pinned = data_dir()
            .ok()
            .and_then(|d| std::fs::read_to_string(d.join(OVERRIDE_FILE)).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| s == "file" || s == "keyring");

        if kind() == Kind::Keyring {
            return BackendStatus {
                kind: "keyring".into(),
                label: "System keyring".into(),
                detail: "Secrets are stored in your desktop's keyring over the D-Bus Secret Service.".into(),
                locked: false,
                uninitialized: false,
                store_file_exists: exists,
                pinned,
                linux: true,
            };
        }
        let unlocked = store().lock().unwrap().is_some();
        BackendStatus {
            kind: "file".into(),
            label: "Encrypted file".into(),
            detail: if !exists {
                "No secret storage service is running in this session. Conduit can keep secrets in an encrypted file instead — you will be asked to choose a passphrase."
            } else if unlocked {
                "Secrets are stored in an encrypted file, unlocked for this session."
            } else {
                "Secrets are stored in an encrypted file. Enter its passphrase to unlock it."
            }
            .into(),
            locked: exists && !unlocked,
            uninitialized: !exists,
            store_file_exists: exists,
            pinned,
            linux: true,
        }
    }
}

/// Set up the encrypted file store for the first time.
#[tauri::command]
pub fn secret_store_create(passphrase: String) -> Result<BackendStatus, String> {
    #[cfg(not(target_os = "linux"))]
    {
        let _ = passphrase;
        Err("this platform uses the system keyring".into())
    }

    #[cfg(target_os = "linux")]
    {
        if passphrase.is_empty() {
            return Err("choose a passphrase".into());
        }
        let s = crate::filestore::create(data_dir()?, &passphrase)?;
        *store().lock().unwrap() = Some(s);
        Ok(secret_backend_status())
    }
}

/// Unlock an existing encrypted file store for this session.
#[tauri::command]
pub fn secret_store_unlock(passphrase: String) -> Result<BackendStatus, String> {
    #[cfg(not(target_os = "linux"))]
    {
        let _ = passphrase;
        Err("this platform uses the system keyring".into())
    }

    #[cfg(target_os = "linux")]
    {
        let s = crate::filestore::unlock(data_dir()?, &passphrase)?;
        *store().lock().unwrap() = Some(s);
        // A previous locked read may have answered "no secret" for entries that
        // do exist. Drop those answers now that the real store is open.
        cache().lock().unwrap().clear();
        Ok(secret_backend_status())
    }
}

// ---------------------------------------------------------------------------
// Read / write
// ---------------------------------------------------------------------------

/// Outcome of a read, distinguishing "there is no such secret" from "the store
/// could not answer". Only the former may be memoized — caching a failure would
/// make one transient hiccup look like a missing password for the whole run.
enum Read {
    Found(String),
    Absent,
    Unavailable,
}

#[cfg(target_os = "linux")]
const LOCKED: &str =
    "the encrypted secret store is locked — unlock it from Secret storage in the sidebar";

fn read(account: &str) -> Read {
    match kind() {
        Kind::Keyring => match entry(account).map(|e| e.get_password()) {
            Ok(Ok(v)) => Read::Found(v),
            Ok(Err(keyring::Error::NoEntry)) => Read::Absent,
            _ => Read::Unavailable,
        },
        #[cfg(target_os = "linux")]
        Kind::File => match store().lock().unwrap().as_ref() {
            Some(s) => match s.get(account) {
                Some(v) => Read::Found(v),
                None => Read::Absent,
            },
            // Locked or not yet set up — we genuinely do not know.
            None => Read::Unavailable,
        },
    }
}

/// Backend-internal read — deliberately NOT a command, so the webview can never
/// pull a plaintext secret back out. Cached after the first successful read.
pub fn get(account: &str) -> Option<String> {
    if let Some(hit) = cache().lock().unwrap().get(account) {
        return hit.clone();
    }
    match read(account) {
        Read::Found(v) => {
            cache().lock().unwrap().insert(account.to_string(), Some(v.clone()));
            Some(v)
        }
        Read::Absent => {
            cache().lock().unwrap().insert(account.to_string(), None);
            None
        }
        // Do not cache: the store may be unlocked later in this same run.
        Read::Unavailable => None,
    }
}

fn write(account: &str, secret: &str) -> Result<(), String> {
    match kind() {
        Kind::Keyring => entry(account)?
            .set_password(secret)
            .map_err(|e| format!("could not save the secret: {}", describe(&e))),
        #[cfg(target_os = "linux")]
        Kind::File => match store().lock().unwrap().as_mut() {
            Some(s) => s.set(account, secret),
            None => Err(LOCKED.into()),
        },
    }
}

fn erase(account: &str) -> Result<(), String> {
    match kind() {
        Kind::Keyring => match entry(account)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(format!("could not remove the secret: {}", describe(&e))),
        },
        #[cfg(target_os = "linux")]
        Kind::File => match store().lock().unwrap().as_mut() {
            Some(s) => s.delete(account),
            None => Err(LOCKED.into()),
        },
    }
}

/// Best-effort delete used by host cleanup; a missing entry is not an error.
pub fn delete(account: &str) {
    let _ = erase(account);
    cache().lock().unwrap().remove(account);
}

#[tauri::command]
pub fn secret_set(host_id: String, secret: String) -> Result<(), String> {
    write(&host_id, &secret)?;
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
    erase(&host_id)
}

/// Best-effort removal of password slots for the given host ids — used to clear
/// junk empty entries an earlier bug wrote for key/telnet hosts. Delete never
/// decrypts, so this raises no Keychain prompt. Missing entries are ignored, so
/// the count is "ids the store accepted", not "entries that existed"; the caller
/// discards it either way.
#[tauri::command]
pub fn secrets_purge(host_ids: Vec<String>) -> usize {
    host_ids
        .into_iter()
        .filter(|id| {
            cache().lock().unwrap().remove(id);
            erase(id).is_ok()
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression test for the bug that started this: the Keys page used to
    /// render `Platform secure storage failure: DBus error: The name
    /// org.freedesktop.secrets was not provided by any .service files` verbatim.
    #[test]
    fn missing_secret_service_becomes_a_human_sentence() {
        let raw = "DBus error: The name org.freedesktop.secrets was not provided by any .service files";
        let e = keyring::Error::PlatformFailure(Box::new(std::io::Error::other(raw)));
        let msg = describe(&e);
        assert_eq!(msg, "no secret storage service is running in this desktop session");
        assert!(!msg.contains("org.freedesktop"));
        assert!(!msg.contains("DBus"));
    }

    #[test]
    fn a_locked_keyring_reads_differently_from_an_absent_entry() {
        assert!(describe(&keyring::Error::NoEntry).contains("no secret stored"));
        let locked = keyring::Error::NoStorageAccess(Box::new(std::io::Error::other("locked")));
        assert!(describe(&locked).contains("locked"));
    }

    /// Unrecognised platform failures still get a prefix that says where the
    /// problem is, rather than leaking as a bare backend string.
    #[test]
    fn unknown_platform_failures_stay_attributed() {
        let e = keyring::Error::PlatformFailure(Box::new(std::io::Error::other("wallet on fire")));
        assert_eq!(describe(&e), "the system keyring is not working: wallet on fire");
    }

    /// The whole reported flow, with no Secret Service: what used to be a hard
    /// failure now sets up, stores, reads back, and reports honestly.
    ///
    /// `KIND` and `DATA_DIR` are process-global OnceLocks, so this must be the
    /// only test that drives a backend — the others above only call `describe`.
    #[cfg(target_os = "linux")]
    #[test]
    fn no_secret_service_falls_back_to_the_file_store() {
        std::env::set_var("CONDUIT_SECRET_BACKEND", "file");
        let dir = std::env::temp_dir().join(format!("conduit-secrets-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let _ = std::fs::remove_file(crate::filestore::path(&dir));
        init_data_dir(dir.clone());

        // Nothing set up yet: the UI is told to ask for a passphrase, and a
        // write fails with a sentence rather than a D-Bus string.
        let s = secret_backend_status();
        assert_eq!(s.kind, "file");
        assert!(s.uninitialized && !s.locked);
        let err = secret_set("key:abc".into(), "PEM".into()).unwrap_err();
        assert!(err.contains("locked"), "{err}");

        // Set a passphrase, then the original operation succeeds.
        secret_store_create("hunter2".into()).unwrap();
        secret_set("key:abc".into(), "PRIVATE KEY".into()).unwrap();
        assert_eq!(get("key:abc").as_deref(), Some("PRIVATE KEY"));
        assert!(secret_has("key:abc".into()));
        assert!(!secret_has("key:nope".into()));
        assert!(secret_backend_status().kind == "file" && !secret_backend_status().locked);

        // Survives a restart: drop the in-process store, and a locked store must
        // not answer "no secret" — that is the cache-poisoning bug.
        *store().lock().unwrap() = None;
        cache().lock().unwrap().clear();
        assert!(secret_backend_status().locked);
        assert_eq!(get("key:abc"), None);
        assert!(
            cache().lock().unwrap().is_empty(),
            "an unavailable store must not be memoized as 'no secret'"
        );

        // Unlocking makes it readable again — which it would not be if the
        // earlier None had been cached.
        secret_store_unlock("hunter2".into()).unwrap();
        assert_eq!(get("key:abc").as_deref(), Some("PRIVATE KEY"));

        secret_delete("key:abc".into()).unwrap();
        assert_eq!(get("key:abc"), None);
        std::fs::remove_dir_all(&dir).ok();
    }
}
