//! Session logging: when a host has logging enabled, its terminal output is
//! teed to a per-session file under the app data dir's `logs/`. Raw PTY bytes
//! (escape codes included) — a faithful transcript. No secrets pass through the
//! output stream, so this is safe to persist.

use std::fs::{self, File, OpenOptions};
use std::path::PathBuf;

use tauri::{AppHandle, Manager};

pub fn logs_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("no app data dir: {e}"))?
        .join("logs");
    fs::create_dir_all(&dir).map_err(|e| format!("create logs dir: {e}"))?;
    Ok(dir)
}

/// Open a fresh append log for a session, named `<sanitized-host>-<unixsecs>.log`.
pub fn open_log(app: &AppHandle, name: &str) -> Option<File> {
    let dir = logs_dir(app).ok()?;
    let safe: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    let path = dir.join(format!("{safe}-{}.log", crate::knownhosts::now_secs()));
    OpenOptions::new().create(true).append(true).open(path).ok()
}

/// Absolute path of the logs directory (for a future "reveal in Finder").
#[tauri::command]
pub fn logs_dir_path(app: AppHandle) -> Result<String, String> {
    Ok(logs_dir(&app)?.to_string_lossy().to_string())
}
