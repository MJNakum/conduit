//! SFTP browser backend (design-spec §15). Each open SFTP session rides its own
//! SSH connection (built non-interactively via ssh::connect_chain, so the host
//! must already be trusted, like forwards) and exposes list/download/upload.
//! Whole-file transfers for now — fine for config files and logs; streaming can
//! come later for very large files.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use russh::client::Handle;
use russh_sftp::client::SftpSession;
use serde::Serialize;
use tauri::{AppHandle, State};

use crate::ssh::Handler;

struct SftpConn {
    sftp: Arc<SftpSession>,
    // Kept alive so the channel transport backing the SFTP stream stays up.
    _handle: Handle<Handler>,
    _bastions: Vec<Handle<Handler>>,
}

#[derive(Default)]
pub struct SftpState {
    conns: Arc<Mutex<HashMap<String, SftpConn>>>,
}

#[derive(Serialize)]
pub struct Entry {
    name: String,
    is_dir: bool,
    size: u64,
}

#[derive(Serialize)]
pub struct Opened {
    id: String,
    cwd: String,
}

static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Pull the shared SftpSession out from under the lock (never hold a std Mutex
/// across .await).
fn session(state: &SftpState, id: &str) -> Result<Arc<SftpSession>, String> {
    state
        .conns
        .lock()
        .unwrap()
        .get(id)
        .map(|c| c.sftp.clone())
        .ok_or_else(|| "no such sftp session".into())
}

#[tauri::command]
pub async fn sftp_open(
    app: AppHandle,
    state: State<'_, SftpState>,
    host_id: String,
) -> Result<Opened, String> {
    let id = format!("f{}", NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
    let chain = crate::ssh::build_chain(&app, &host_id)?;
    let (handle, bastions) = crate::ssh::connect_chain(
        &app,
        &id,
        &chain,
        crate::ssh::dummy_pending(),
        false, // non-interactive: untrusted host key -> reject (connect once first)
        None,
        true,
    )
    .await?;

    let channel = handle
        .channel_open_session()
        .await
        .map_err(|e| format!("open channel: {e}"))?;
    channel
        .request_subsystem(false, "sftp")
        .await
        .map_err(|e| format!("request sftp subsystem: {e}"))?;
    let sftp = SftpSession::new(channel.into_stream())
        .await
        .map_err(|e| format!("sftp handshake: {e}"))?;
    let cwd = sftp.canonicalize(".").await.unwrap_or_else(|_| "/".into());

    state.conns.lock().unwrap().insert(
        id.clone(),
        SftpConn {
            sftp: Arc::new(sftp),
            _handle: handle,
            _bastions: bastions,
        },
    );
    Ok(Opened { id, cwd })
}

#[tauri::command]
pub async fn sftp_list(
    state: State<'_, SftpState>,
    id: String,
    path: String,
) -> Result<Vec<Entry>, String> {
    let sftp = session(&state, &id)?;
    let dir = sftp.read_dir(&path).await.map_err(|e| format!("read dir: {e}"))?;
    let mut out: Vec<Entry> = dir
        .map(|e| {
            let m = e.metadata();
            Entry {
                name: e.file_name(),
                is_dir: m.is_dir(),
                size: m.size.unwrap_or(0),
            }
        })
        .collect();
    // Directories first, then case-insensitive by name.
    out.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(out)
}

#[tauri::command]
pub async fn sftp_download(
    state: State<'_, SftpState>,
    id: String,
    remote: String,
    local: String,
) -> Result<(), String> {
    let sftp = session(&state, &id)?;
    let bytes = sftp.read(&remote).await.map_err(|e| format!("download {remote}: {e}"))?;
    let local = crate::ssh::expand_tilde(&local);
    std::fs::write(&local, bytes).map_err(|e| format!("write {local}: {e}"))
}

#[tauri::command]
pub async fn sftp_upload(
    state: State<'_, SftpState>,
    id: String,
    local: String,
    remote: String,
) -> Result<(), String> {
    let sftp = session(&state, &id)?;
    let local = crate::ssh::expand_tilde(&local);
    let bytes = std::fs::read(&local).map_err(|e| format!("read {local}: {e}"))?;
    sftp.write(&remote, &bytes).await.map_err(|e| format!("upload {remote}: {e}"))
}

#[tauri::command]
pub fn sftp_close(state: State<'_, SftpState>, id: String) {
    state.conns.lock().unwrap().remove(&id);
}
