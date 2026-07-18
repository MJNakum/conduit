//! Phase 0 SSH layer: one russh session per connection, owned by an async task.
//! The webview never touches SSH — it invokes commands and subscribes to the
//! `ssh://data` and `ssh://state` events. State events are driven by *real*
//! connection progress so the Phase 1 live stepper is never a faked animation.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use russh::client;
use russh::keys::{load_secret_key, Algorithm, HashAlg, PrivateKeyWithHashAlg};
use russh::ChannelMsg;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::{mpsc, oneshot};

/// Input flowing from the webview toward the server for one session.
enum SessionInput {
    Data(Vec<u8>),
    Resize { cols: u32, rows: u32 },
}

/// Connection credentials, kept in RAM for the session's lifetime so reconnect
/// can reuse them. The secret (password or key passphrase) lives in RAM only
/// for the active process; its durable home is the OS keychain (`secrets.rs`).
#[derive(Clone)]
struct Creds {
    host: String,
    port: u16,
    user: String,
    auth: String,                 // "password" | "key"
    identity_file: Option<String>, // private-key path when auth == "key"
    secret: String,               // password, or key passphrase ("" if none)
}

/// A tracked session. `tx` is `None` while disconnected — the entry lingers so
/// `ssh_reconnect` can restart it; `ssh_disconnect` drops the entry entirely.
struct Session {
    tx: Option<mpsc::UnboundedSender<SessionInput>>,
    creds: Creds,
}

type Sessions = Arc<Mutex<HashMap<String, Session>>>;
/// Pending host-key Accept/Reject decisions, keyed by session id. `check_server_key`
/// parks a receiver here; `ssh_host_key_decision` from the webview fulfills it.
type Pending = Arc<Mutex<HashMap<String, oneshot::Sender<bool>>>>;

#[derive(Default)]
pub struct SshState {
    // ponytail: single global map guarded by a std Mutex — fine for the handful
    // of sessions a user opens; shard or use dashmap only if that ever bottlenecks.
    sessions: Sessions,
    pending: Pending,
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Real connection state, emitted on `ssh://state`. This is the source of truth
/// for the live connection stepper.
#[derive(Serialize, Clone)]
#[serde(tag = "state", rename_all = "lowercase")]
enum ConnState {
    Connecting,
    // Host-key verification pause: the webview shows the fingerprint and answers
    // via `ssh_host_key_decision`. `changed`/`old` drive the §12 red warning.
    HostKey {
        fingerprint: String,
        key_type: String,
        changed: bool,
        old: Option<String>,
    },
    Authenticating {
        method: String,
    },
    Connected,
    Disconnected {
        reason: Option<String>,
    },
    Error {
        message: String,
    },
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

/// russh client handler. Carries what `check_server_key` needs to run TOFU:
/// where to emit the prompt, which session/host it's for, and how to await the
/// webview's Accept/Reject.
struct Handler {
    app: AppHandle,
    id: String,
    host: String,
    port: u16,
    pending: Pending,
}

impl client::Handler for Handler {
    type Error = russh::Error;

    // TOFU host-key verification. Unknown key -> prompt + wait; known match ->
    // accept; known mismatch -> loud change warning + wait. On Accept we persist
    // the fingerprint to the known-hosts store.
    async fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        let fingerprint = server_public_key.fingerprint(HashAlg::Sha256).to_string();
        let key_type = server_public_key.algorithm().to_string();

        let existing = crate::knownhosts::lookup(&self.app, &self.host, self.port);
        if let Some(k) = &existing {
            if k.fingerprint == fingerprint {
                return Ok(true); // known and unchanged
            }
        }

        // Unknown or changed: park a decision channel, emit the prompt, wait.
        let (tx, rx) = oneshot::channel::<bool>();
        self.pending.lock().unwrap().insert(self.id.clone(), tx);
        emit_state(
            &self.app,
            &self.id,
            ConnState::HostKey {
                fingerprint: fingerprint.clone(),
                key_type: key_type.clone(),
                changed: existing.is_some(),
                old: existing.as_ref().map(|k| k.fingerprint.clone()),
            },
        );
        // ponytail: no timeout — closing the tab drops the session to abort.
        let accepted = rx.await.unwrap_or(false);
        if accepted {
            let _ = crate::knownhosts::upsert(
                &self.app,
                crate::knownhosts::KnownHost {
                    host: self.host.clone(),
                    port: self.port,
                    key_type,
                    fingerprint,
                    added: crate::knownhosts::now_secs(),
                    source: "accepted".into(),
                },
            );
        }
        Ok(accepted)
    }
}

/// Open a session. `secret` is the password or key passphrase entered in the
/// webview; when `None` we fall back to the keychain entry for `host_id`. When
/// `save` is set, the provided secret is written to the keychain.
#[tauri::command]
pub async fn ssh_connect(
    app: AppHandle,
    state: State<'_, SshState>,
    host_id: Option<String>,
    host: String,
    port: u16,
    user: String,
    auth: String,
    identity_file: Option<String>,
    secret: Option<String>,
    save: bool,
) -> Result<String, String> {
    let secret = match secret {
        Some(s) => {
            if save {
                if let Some(hid) = &host_id {
                    let _ = crate::secrets::secret_set(hid.clone(), s.clone());
                }
            }
            s
        }
        None => host_id
            .as_ref()
            .and_then(|h| crate::secrets::get(h))
            .unwrap_or_default(),
    };

    let id = format!("s{}", NEXT_ID.fetch_add(1, Ordering::Relaxed));
    state.sessions.lock().unwrap().insert(
        id.clone(),
        Session {
            tx: None,
            creds: Creds {
                host,
                port,
                user,
                auth,
                identity_file,
                secret,
            },
        },
    );
    start_session(app, id.clone(), state.sessions.clone(), state.pending.clone());
    Ok(id)
}

/// Restart a session in place (same id), reusing its retained credentials.
#[tauri::command]
pub fn ssh_reconnect(app: AppHandle, state: State<'_, SshState>, id: String) -> Result<(), String> {
    {
        let sessions = state.sessions.lock().unwrap();
        let s = sessions.get(&id).ok_or("no such session")?;
        if s.tx.is_some() {
            return Ok(()); // already connected — nothing to do
        }
    }
    start_session(app, id, state.sessions.clone(), state.pending.clone());
    Ok(())
}

/// Fulfill a parked host-key decision from the webview (Accept = true).
#[tauri::command]
pub fn ssh_host_key_decision(state: State<'_, SshState>, id: String, accept: bool) {
    if let Some(tx) = state.pending.lock().unwrap().remove(&id) {
        let _ = tx.send(accept);
    }
}

/// Spawn the async task that owns the russh session for `id`, wiring a fresh
/// input channel. Reads the retained creds from the map.
fn start_session(app: AppHandle, id: String, sessions: Sessions, pending: Pending) {
    let creds = match sessions.lock().unwrap().get(&id) {
        Some(s) => s.creds.clone(),
        None => return,
    };
    let (tx, rx) = mpsc::unbounded_channel::<SessionInput>();
    match sessions.lock().unwrap().get_mut(&id) {
        Some(s) => s.tx = Some(tx),
        None => return,
    }

    tauri::async_runtime::spawn(async move {
        let result = run_session(&app, &id, creds, rx, pending.clone()).await;
        pending.lock().unwrap().remove(&id); // clear any un-answered prompt
        // Never leak credentials into the error surfaced to the UI.
        let final_state = match result {
            Ok(reason) => ConnState::Disconnected { reason },
            Err(message) => ConnState::Error { message },
        };
        emit_state(&app, &id, final_state);
        // Retain the entry (and creds) so the user can reconnect; just mark the
        // sender gone. ssh_disconnect removes the entry outright.
        if let Some(s) = sessions.lock().unwrap().get_mut(&id) {
            s.tx = None;
        }
    });
}

/// Owns the russh session for its whole lifetime. Returns an optional
/// disconnect reason on clean exit, or an error string on failure.
async fn run_session(
    app: &AppHandle,
    id: &str,
    creds: Creds,
    mut rx: mpsc::UnboundedReceiver<SessionInput>,
    pending: Pending,
) -> Result<Option<String>, String> {
    let Creds {
        host,
        port,
        user,
        auth,
        identity_file,
        secret,
    } = creds;

    emit_state(app, id, ConnState::Connecting);

    let config = Arc::new(client::Config::default());
    let handler = Handler {
        app: app.clone(),
        id: id.to_string(),
        host: host.clone(),
        port,
        pending,
    };
    let mut handle = client::connect(config, (host.as_str(), port), handler)
        .await
        .map_err(|e| format!("connect failed: {e}"))?;

    let method = if auth == "key" { "publickey" } else { "password" };
    emit_state(
        app,
        id,
        ConnState::Authenticating {
            method: method.into(),
        },
    );

    let result = if auth == "key" {
        let path = identity_file.ok_or("no identity file set for key auth")?;
        let path = expand_tilde(&path);
        let passphrase = (!secret.is_empty()).then_some(secret.as_str());
        let key = load_secret_key(&path, passphrase).map_err(|e| format!("load key: {e}"))?;
        // RSA keys need an explicit SHA-2 hash alg (rsa-sha2-256); others ignore it.
        let hash = matches!(key.algorithm(), Algorithm::Rsa { .. }).then_some(HashAlg::Sha256);
        handle
            .authenticate_publickey(&user, PrivateKeyWithHashAlg::new(Arc::new(key), hash))
            .await
    } else {
        handle.authenticate_password(&user, &secret).await
    };
    let auth_result = result.map_err(|e| format!("auth error: {e}"))?;
    if !matches!(auth_result, client::AuthResult::Success) {
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
    let tx = session_tx(&sessions, &id)?;
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
    let tx = session_tx(&sessions, &id)?;
    tx.send(SessionInput::Resize { cols, rows })
        .map_err(|_| "session closed".to_string())
}

/// Expand a leading `~/` to $HOME. ssh_config IdentityFile paths use it; russh
/// does not. ponytail: only the leading `~`, which is the case that occurs.
fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{home}/{rest}");
        }
    }
    path.to_string()
}

fn session_tx<'a>(
    sessions: &'a HashMap<String, Session>,
    id: &str,
) -> Result<&'a mpsc::UnboundedSender<SessionInput>, String> {
    sessions
        .get(id)
        .ok_or("no such session")?
        .tx
        .as_ref()
        .ok_or_else(|| "session not connected".to_string())
}

/// Drop the session: removing its sender ends the select loop (rx yields None),
/// which closes the channel and lets the task exit cleanly.
#[tauri::command]
pub fn ssh_disconnect(state: State<'_, SshState>, id: String) {
    state.sessions.lock().unwrap().remove(&id);
}

#[cfg(test)]
mod tests {
    use super::expand_tilde;

    #[test]
    fn tilde_expands_only_leading() {
        std::env::set_var("HOME", "/Users/x");
        assert_eq!(expand_tilde("~/.ssh/id_ed25519"), "/Users/x/.ssh/id_ed25519");
        // Absolute and bare-tilde paths pass through untouched.
        assert_eq!(expand_tilde("/etc/ssh/key"), "/etc/ssh/key");
        assert_eq!(expand_tilde("~root/key"), "~root/key");
    }
}
