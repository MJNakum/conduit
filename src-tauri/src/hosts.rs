//! Saved-host store. Phase 1: a single `hosts.json` in the app data dir.
//! Import/export to standard `ssh_config` is Phase 3 — the fields here map
//! 1:1 so that stays trivial. No new deps: serde + std::fs.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Serialize, Deserialize, Clone)]
pub struct Host {
    pub id: String,
    pub name: String,
    pub hostname: String,
    pub port: u16,
    pub user: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default, rename = "autoReconnect")]
    pub auto_reconnect: bool,
    // Auth: "password" | "key". Secrets never live here — only in the keychain.
    #[serde(default = "default_auth")]
    pub auth: String,
    // Managed key (Key Manager) id, when auth == "key". Secret stays in Keychain.
    #[serde(default, rename = "keyId")]
    pub key_id: Option<String>,
    // ssh_config-native IdentityFile path; used when auth == "key" without a managed key.
    #[serde(default, rename = "identityFile")]
    pub identity_file: Option<String>,
    // Ordered saved-host ids to ProxyJump through (bastion-1 … target).
    #[serde(default)]
    pub jumps: Vec<String>,
    // Verbatim ssh_config option lines we don't model, preserved for lossless
    // round-trip on export (each like "ForwardAgent yes").
    #[serde(default)]
    pub raw: Vec<String>,
    // Per-connection terminal theming (design-spec §18). null => use the global
    // default. Presentation-only; the webview owns the scheme catalog.
    #[serde(default)]
    pub scheme: Option<String>,
    #[serde(default)]
    pub font: Option<String>,
    #[serde(default, rename = "fontSize")]
    pub font_size: Option<u16>,
}

fn default_auth() -> String {
    "password".into()
}

fn store_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("no app data dir: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("create data dir: {e}"))?;
    Ok(dir.join("hosts.json"))
}

/// A default Host with the given id/name, for importers (ssh_config, PuTTY).
pub(crate) fn blank_host(id: &str, name: &str) -> Host {
    Host {
        id: id.into(),
        name: name.into(),
        hostname: String::new(),
        port: 22,
        user: String::new(),
        tags: Vec::new(),
        color: None,
        favorite: false,
        group: None,
        auto_reconnect: false,
        auth: "password".into(),
        key_id: None,
        identity_file: None,
        jumps: Vec::new(),
        raw: Vec::new(),
        scheme: None,
        font: None,
        font_size: None,
    }
}

fn read_all(app: &AppHandle) -> Result<Vec<Host>, String> {
    let path = store_path(app)?;
    match fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| format!("parse hosts.json: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(format!("read hosts.json: {e}")),
    }
}

fn write_all(app: &AppHandle, hosts: &[Host]) -> Result<(), String> {
    let path = store_path(app)?;
    let json = serde_json::to_vec_pretty(hosts).map_err(|e| format!("serialize hosts: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("write hosts.json: {e}"))
}

#[tauri::command]
pub fn hosts_list(app: AppHandle) -> Result<Vec<Host>, String> {
    read_all(&app)
}

#[tauri::command]
pub fn host_save(app: AppHandle, host: Host) -> Result<Host, String> {
    let mut hosts = read_all(&app)?;
    match hosts.iter_mut().find(|h| h.id == host.id) {
        Some(existing) => *existing = host.clone(),
        None => hosts.push(host.clone()),
    }
    write_all(&app, &hosts)?;
    Ok(host)
}

#[tauri::command]
pub fn host_delete(app: AppHandle, id: String) -> Result<(), String> {
    let mut hosts = read_all(&app)?;
    hosts.retain(|h| h.id != id);
    crate::secrets::delete(&id); // drop any keychain secret for this host
    write_all(&app, &hosts)
}
