//! Saved command snippets (design-spec §16). Plain content, no secrets — stored
//! in `snippets.json` in the app data dir (mirrors hosts.rs). Parameterized
//! `{{var}}` substitution and "run into the active session" are handled in the
//! webview; the backend is just the store.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Serialize, Deserialize, Clone)]
pub struct Snippet {
    pub id: String,
    pub name: String,
    pub command: String,
    // Prompt before running (for loaded-gun commands).
    #[serde(default)]
    pub confirm: bool,
}

fn store_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("no app data dir: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("create data dir: {e}"))?;
    Ok(dir.join("snippets.json"))
}

fn read_all(app: &AppHandle) -> Result<Vec<Snippet>, String> {
    let path = store_path(app)?;
    match fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| format!("parse snippets.json: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(format!("read snippets.json: {e}")),
    }
}

fn write_all(app: &AppHandle, all: &[Snippet]) -> Result<(), String> {
    let path = store_path(app)?;
    let json = serde_json::to_vec_pretty(all).map_err(|e| format!("serialize: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("write snippets.json: {e}"))
}

#[tauri::command]
pub fn snippets_list(app: AppHandle) -> Result<Vec<Snippet>, String> {
    read_all(&app)
}

#[tauri::command]
pub fn snippet_save(app: AppHandle, snippet: Snippet) -> Result<Snippet, String> {
    let mut all = read_all(&app)?;
    match all.iter_mut().find(|s| s.id == snippet.id) {
        Some(existing) => *existing = snippet.clone(),
        None => all.push(snippet.clone()),
    }
    write_all(&app, &all)?;
    Ok(snippet)
}

#[tauri::command]
pub fn snippet_delete(app: AppHandle, id: String) -> Result<(), String> {
    let mut all = read_all(&app)?;
    all.retain(|s| s.id != id);
    write_all(&app, &all)
}
