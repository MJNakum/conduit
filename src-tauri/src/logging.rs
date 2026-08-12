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

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect()
}

/// Open a fresh append log for a session, named `<sanitized-host>-<unixsecs>.log`.
pub fn open_log(app: &AppHandle, name: &str) -> Option<File> {
    let dir = logs_dir(app).ok()?;
    let path = dir.join(format!("{}-{}.log", sanitize(name), crate::knownhosts::now_secs()));
    OpenOptions::new().create(true).append(true).open(path).ok()
}

/// Save a connection log — the stepper's diagnostic trace, not terminal output —
/// and return its bare filename so the caller can hand it to `log_reveal`.
///
/// Named `<host>-connect-<unixsecs>.log` rather than `<host>-<secs>-connect.log`
/// so `logs_list`'s `<name>-<secs>` split still finds the timestamp; the file
/// then lists alongside session logs under the host label `<host>-connect`.
///
/// `text` is composed in the webview from `ssh://log` lines, which carry prompt
/// text but never an answer or a secret (CLAUDE.md), so it is safe to persist.
#[tauri::command]
pub fn conn_log_save(app: AppHandle, name: String, text: String) -> Result<String, String> {
    let file = format!("{}-connect-{}.log", sanitize(&name), crate::knownhosts::now_secs());
    fs::write(logs_dir(&app)?.join(&file), text).map_err(|e| format!("write log: {e}"))?;
    Ok(file)
}

/// Absolute path of the logs directory (for the "reveal in file manager" action).
#[tauri::command]
pub fn logs_dir_path(app: AppHandle) -> Result<String, String> {
    Ok(logs_dir(&app)?.to_string_lossy().to_string())
}

#[derive(serde::Serialize)]
pub struct LogEntry {
    file: String, // bare filename, the handle passed back to read/reveal
    host: String, // decoded from the `<host>-<unixsecs>.log` name
    ts: u64,      // unix seconds the session started
    size: u64,    // bytes
}

/// List saved session logs, newest first. Filenames are `<host>-<secs>.log`.
#[tauri::command]
pub fn logs_list(app: AppHandle) -> Result<Vec<LogEntry>, String> {
    let dir = logs_dir(&app)?;
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| format!("read logs dir: {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        let Some(stem) = name.strip_suffix(".log") else { continue };
        let (host, ts) = stem
            .rsplit_once('-')
            .map(|(h, t)| (h.to_string(), t.parse::<u64>().unwrap_or(0)))
            .unwrap_or((stem.to_string(), 0));
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        out.push(LogEntry { file: name, host, ts, size });
    }
    out.sort_by(|a, b| b.ts.cmp(&a.ts));
    Ok(out)
}

// Reject anything that could escape the logs dir — we only ever open our own files.
fn safe_in_logs(app: &AppHandle, file: &str) -> Result<PathBuf, String> {
    if file.contains('/') || file.contains('\\') || file.contains("..") {
        return Err("invalid log name".into());
    }
    Ok(logs_dir(app)?.join(file))
}

/// Read a saved log. Caps the returned text to the last 512 KB so a huge
/// transcript can't stall the webview; prepends a notice when truncated.
#[tauri::command]
pub fn log_read(app: AppHandle, file: String) -> Result<String, String> {
    const CAP: u64 = 512 * 1024;
    let path = safe_in_logs(&app, &file)?;
    let bytes = fs::read(&path).map_err(|e| format!("read log: {e}"))?;
    let len = bytes.len() as u64;
    let slice = if len > CAP { &bytes[(len - CAP) as usize..] } else { &bytes[..] };
    let text = String::from_utf8_lossy(slice).to_string();
    Ok(if len > CAP { format!("… (showing last 512 KB of {len} bytes)\n\n{text}") } else { text })
}

/// Reveal a log (or the logs folder when `file` is None) in the system file manager.
/// Uses `open -R` on macOS, `explorer /select` on Windows, `xdg-open` elsewhere.
#[tauri::command]
pub fn log_reveal(app: AppHandle, file: Option<String>) -> Result<(), String> {
    let target = match file {
        Some(f) => safe_in_logs(&app, &f)?,
        None => logs_dir(&app)?,
    };
    reveal_path(&target)
}

#[cfg(target_os = "macos")]
fn reveal_path(target: &std::path::Path) -> Result<(), String> {
    let mut cmd = std::process::Command::new("open");
    if target.is_file() {
        cmd.arg("-R");
    }
    cmd.arg(target).spawn().map_err(|e| format!("open: {e}"))?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn reveal_path(target: &std::path::Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    // /select highlights the file inside its parent folder; for dirs just open them.
    let arg = if target.is_file() {
        format!("/select,{}", target.display())
    } else {
        target.display().to_string()
    };
    // CREATE_NO_WINDOW suppresses the brief console flash that explorer spawns.
    std::process::Command::new("explorer.exe")
        .raw_arg(&arg)
        .creation_flags(0x0800_0000)
        .spawn()
        .map_err(|e| format!("explorer: {e}"))?;
    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn reveal_path(target: &std::path::Path) -> Result<(), String> {
    let dir = if target.is_file() { target.parent().unwrap_or(target) } else { target };
    std::process::Command::new("xdg-open")
        .arg(dir)
        .spawn()
        .map_err(|e| format!("xdg-open: {e}"))?;
    Ok(())
}

/// Delete a saved log.
#[tauri::command]
pub fn log_delete(app: AppHandle, file: String) -> Result<(), String> {
    fs::remove_file(safe_in_logs(&app, &file)?).map_err(|e| format!("delete log: {e}"))
}
