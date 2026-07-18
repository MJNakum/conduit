//! Known-host fingerprint store (TOFU). A single `known_hosts.json` in the app
//! data dir, mirroring the whole-file read/write pattern of `hosts.rs`. Richer
//! metadata than an OpenSSH `known_hosts` line (source, added-date) for the
//! design-spec §12 management table; import/export to standard `known_hosts` is
//! a later pass. Verification + change warnings are driven from `ssh.rs`.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Serialize, Deserialize, Clone)]
pub struct KnownHost {
    pub host: String,
    pub port: u16,
    pub key_type: String,
    pub fingerprint: String, // "SHA256:<base64>"
    pub added: String,       // unix seconds as string; good enough for a date column
    pub source: String,      // "accepted" | "imported"
}

fn store_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("no app data dir: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("create data dir: {e}"))?;
    Ok(dir.join("known_hosts.json"))
}

pub fn read_all(app: &AppHandle) -> Result<Vec<KnownHost>, String> {
    let path = store_path(app)?;
    match fs::read(&path) {
        Ok(bytes) => {
            serde_json::from_slice(&bytes).map_err(|e| format!("parse known_hosts.json: {e}"))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(format!("read known_hosts.json: {e}")),
    }
}

fn write_all(app: &AppHandle, entries: &[KnownHost]) -> Result<(), String> {
    let path = store_path(app)?;
    let json = serde_json::to_vec_pretty(entries).map_err(|e| format!("serialize: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("write known_hosts.json: {e}"))
}

/// The stored entry for a host:port, if any.
pub fn lookup(app: &AppHandle, host: &str, port: u16) -> Option<KnownHost> {
    read_all(app)
        .ok()?
        .into_iter()
        .find(|k| k.host == host && k.port == port)
}

/// Insert or replace the entry for a host:port (called on Accept).
pub fn upsert(app: &AppHandle, entry: KnownHost) -> Result<(), String> {
    let mut all = read_all(app)?;
    match all
        .iter_mut()
        .find(|k| k.host == entry.host && k.port == entry.port)
    {
        Some(existing) => *existing = entry,
        None => all.push(entry),
    }
    write_all(app, &all)
}

/// Unix seconds as a string, for the `added` column. No date crate needed.
pub fn now_secs() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default()
}

/// List command for a future §12 management table.
#[tauri::command]
pub fn known_hosts_list(app: AppHandle) -> Result<Vec<KnownHost>, String> {
    read_all(&app)
}
