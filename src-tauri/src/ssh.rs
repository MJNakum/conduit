//! Phase 0 SSH layer: one russh session per connection, owned by an async task.
//! The webview never touches SSH — it invokes commands and subscribes to the
//! `ssh://data` and `ssh://state` events. State events are driven by *real*
//! connection progress so the Phase 1 live stepper is never a faked animation.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use russh::client;
use russh::ChannelMsg;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::mpsc;

/// Input flowing from the webview toward the server for one session.
enum SessionInput {
    Data(Vec<u8>),
    Resize { cols: u32, rows: u32 },
}

#[derive(Default)]
pub struct SshState {
    // ponytail: single global map guarded by a std Mutex — fine for the handful
    // of sessions a user opens; shard or use dashmap only if that ever bottlenecks.
    sessions: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<SessionInput>>>>,
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Real connection state, emitted on `ssh://state`. This is the source of truth
/// for the live connection stepper.
#[derive(Serialize, Clone)]
#[serde(tag = "state", rename_all = "lowercase")]
enum ConnState {
    Connecting,
    Authenticating { method: String },
    Connected,
    Disconnected { reason: Option<String> },
    Error { message: String },
}

#[derive(Serialize, Clone)]
struct StateEvent {
    id: String,
    #[serde(flatten)]
    state: ConnState,
}

#[derive(Serialize, Clone)]
struct DataEvent {
    id: String,
    bytes: Vec<u8>,
}

fn emit_state(app: &AppHandle, id: &str, state: ConnState) {
    let _ = app.emit(
        "ssh://state",
        StateEvent {
            id: id.to_string(),
            state,
        },
    );
}

/// russh client handler. Phase 0 accepts every host key.
struct Handler;

impl client::Handler for Handler {
    type Error = russh::Error;

    // ponytail: accept-and-forget host key. Known-hosts verification + change
    // warnings are Phase 2 — this is the only spot that needs to change.
    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

#[tauri::command]
pub async fn ssh_connect(
    app: AppHandle,
    state: State<'_, SshState>,
    host: String,
    port: u16,
    user: String,
    password: String,
) -> Result<String, String> {
    let id = format!("s{}", NEXT_ID.fetch_add(1, Ordering::Relaxed));
    let (tx, rx) = mpsc::unbounded_channel::<SessionInput>();

    state
        .sessions
        .lock()
        .unwrap()
        .insert(id.clone(), tx);

    let sessions = state.sessions.clone();
    let task_id = id.clone();
    tauri::async_runtime::spawn(async move {
        let result = run_session(&app, &task_id, host, port, user, password, rx).await;
        // Never leak credentials into the error surfaced to the UI.
        let final_state = match result {
            Ok(reason) => ConnState::Disconnected { reason },
            Err(message) => ConnState::Error { message },
        };
        emit_state(&app, &task_id, final_state);
        sessions.lock().unwrap().remove(&task_id);
    });

    Ok(id)
}

/// Owns the russh session for its whole lifetime. Returns an optional
/// disconnect reason on clean exit, or an error string on failure.
async fn run_session(
    app: &AppHandle,
    id: &str,
    host: String,
    port: u16,
    user: String,
    password: String,
    mut rx: mpsc::UnboundedReceiver<SessionInput>,
) -> Result<Option<String>, String> {
    emit_state(app, id, ConnState::Connecting);

    let config = Arc::new(client::Config::default());
    let mut handle = client::connect(config, (host.as_str(), port), Handler)
        .await
        .map_err(|e| format!("connect failed: {e}"))?;

    emit_state(
        app,
        id,
        ConnState::Authenticating {
            method: "password".into(),
        },
    );

    let auth = handle
        .authenticate_password(&user, &password)
        .await
        .map_err(|e| format!("auth error: {e}"))?;
    if !matches!(auth, client::AuthResult::Success) {
        return Err("authentication failed".into());
    }

    let mut channel = handle
        .channel_open_session()
        .await
        .map_err(|e| format!("channel open failed: {e}"))?;

    // Default PTY size; the frontend sends a real size right after connecting.
    channel
        .request_pty(false, "xterm-256color", 80, 24, 0, 0, &[])
        .await
        .map_err(|e| format!("pty request failed: {e}"))?;
    channel
        .request_shell(false)
        .await
        .map_err(|e| format!("shell request failed: {e}"))?;

    emit_state(app, id, ConnState::Connected);

    // Single loop reads from the server and writes webview input, avoiding a
    // channel split. Add broadcast fan-out here later (one input → many rx).
    loop {
        tokio::select! {
            msg = channel.wait() => match msg {
                Some(ChannelMsg::Data { data }) => {
                    let _ = app.emit("ssh://data", DataEvent {
                        id: id.to_string(),
                        bytes: data.to_vec(),
                    });
                }
                Some(ChannelMsg::ExitStatus { exit_status }) => {
                    return Ok(Some(format!("exited with status {exit_status}")));
                }
                Some(ChannelMsg::Eof) | None => {
                    return Ok(None);
                }
                _ => {}
            },
            input = rx.recv() => match input {
                Some(SessionInput::Data(bytes)) => {
                    channel.data(&bytes[..]).await
                        .map_err(|e| format!("write failed: {e}"))?;
                }
                Some(SessionInput::Resize { cols, rows }) => {
                    let _ = channel.window_change(cols, rows, 0, 0).await;
                }
                None => return Ok(None), // session dropped from the map
            },
        }
    }
}

#[tauri::command]
pub fn ssh_write(state: State<'_, SshState>, id: String, data: String) -> Result<(), String> {
    let sessions = state.sessions.lock().unwrap();
    let tx = sessions.get(&id).ok_or("no such session")?;
    tx.send(SessionInput::Data(data.into_bytes()))
        .map_err(|_| "session closed".to_string())
}

#[tauri::command]
pub fn ssh_resize(
    state: State<'_, SshState>,
    id: String,
    cols: u32,
    rows: u32,
) -> Result<(), String> {
    let sessions = state.sessions.lock().unwrap();
    let tx = sessions.get(&id).ok_or("no such session")?;
    tx.send(SessionInput::Resize { cols, rows })
        .map_err(|_| "session closed".to_string())
}
