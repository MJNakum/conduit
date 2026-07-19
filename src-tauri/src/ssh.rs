//! Phase 0 SSH layer: one russh session per connection, owned by an async task.
//! The webview never touches SSH — it invokes commands and subscribes to the
//! `ssh://data` and `ssh://state` events. State events are driven by *real*
//! connection progress so the Phase 1 live stepper is never a faked animation.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use russh::client;
use russh::keys::{decode_secret_key, load_secret_key, Algorithm, HashAlg, PrivateKeyWithHashAlg};
use russh::ChannelMsg;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::{mpsc, oneshot};

/// Input flowing from the webview toward the server for one session.
pub(crate) enum SessionInput {
    Data(Vec<u8>),
    Resize { cols: u32, rows: u32 },
}

/// One node in a connection path. For a direct connection there's a single hop
/// (the target); with ProxyJump the chain is [bastion-1, …, target]. Secrets
/// live in RAM only for the active process; their durable home is the keychain.
#[derive(Clone)]
pub(crate) struct Hop {
    host: String,
    port: u16,
    user: String,
    auth: String,                  // "password" | "key"
    key_id: Option<String>,        // managed key (keychain) when auth == "key"
    identity_file: Option<String>, // else a private-key path
    secret: String,                // password, or key passphrase ("" if none)
}

/// How a session reaches its host: an SSH hop chain, or a raw Telnet socket.
#[derive(Clone)]
enum Transport {
    Ssh(Vec<Hop>),
    Telnet(String, u16),
}

/// A tracked session. `tx` is `None` while disconnected — the entry lingers so
/// `ssh_reconnect` can restart it; `ssh_disconnect` drops the entry entirely.
struct Session {
    tx: Option<mpsc::UnboundedSender<SessionInput>>,
    transport: Transport,
    // Host name to log this session's output under, or None if logging is off.
    log_name: Option<String>,
}

/// A jump hop supplied by the webview (references a saved host). Its secret is
/// resolved from the keychain by `host_id` at connect time.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JumpInput {
    host_id: Option<String>,
    host: String,
    port: u16,
    user: String,
    auth: String,
    key_id: Option<String>,
    identity_file: Option<String>,
}

type Sessions = Arc<Mutex<HashMap<String, Session>>>;
/// Pending host-key Accept/Reject decisions, keyed by session id. `check_server_key`
/// parks a receiver here; `ssh_host_key_decision` from the webview fulfills it.
pub(crate) type Pending = Arc<Mutex<HashMap<String, oneshot::Sender<bool>>>>;

/// A throwaway pending-map for non-interactive connects (forwards) that never prompt.
pub(crate) fn dummy_pending() -> Pending {
    Arc::new(Mutex::new(HashMap::new()))
}

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
    // `host` names the machine being verified (a bastion, mid-chain, isn't the tab's host).
    HostKey {
        host: String,
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

// Helpers so non-SSH transports (telnet) drive the same events/UI.
pub(crate) fn emit_connecting(app: &AppHandle, id: &str) {
    emit_state(app, id, ConnState::Connecting);
}
pub(crate) fn emit_connected(app: &AppHandle, id: &str) {
    emit_state(app, id, ConnState::Connected);
}
pub(crate) fn emit_data(app: &AppHandle, id: &str, bytes: Vec<u8>) {
    let _ = app.emit("ssh://data", DataEvent { id: id.to_string(), bytes });
}

/// russh client handler. Carries what `check_server_key` needs to run TOFU:
/// where to emit the prompt, which session/host it's for, and how to await the
/// webview's Accept/Reject.
pub(crate) struct Handler {
    app: AppHandle,
    id: String,
    host: String,
    port: u16,
    pending: Pending,
    // Interactive sessions prompt for unknown/changed host keys; background
    // forwards can't, so they reject instead (require a prior trusted connect).
    interactive: bool,
    // For remote (-R) forwards: where inbound forwarded channels are piped to.
    forward_target: Option<(String, u16)>,
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

        // Non-interactive (forwards): never block on a UI prompt — reject an
        // untrusted key. The user connects the host once to establish trust.
        if !self.interactive {
            return Ok(false);
        }

        // Unknown or changed: park a decision channel, emit the prompt, wait.
        let (tx, rx) = oneshot::channel::<bool>();
        self.pending.lock().unwrap().insert(self.id.clone(), tx);
        emit_state(
            &self.app,
            &self.id,
            ConnState::HostKey {
                host: self.host.clone(),
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

    // Remote (-R) forward delivery: the server opens a channel per inbound
    // connection to the forwarded port. Accept it and pipe to the local target.
    async fn server_channel_open_forwarded_tcpip(
        &mut self,
        channel: russh::Channel<russh::client::Msg>,
        _connected_address: &str,
        _connected_port: u32,
        _originator_address: &str,
        _originator_port: u32,
        reply: russh::client::ChannelOpenHandle,
        _session: &mut russh::client::Session,
    ) -> Result<(), Self::Error> {
        let _ = reply.accept().await;
        if let Some((host, port)) = self.forward_target.clone() {
            tokio::spawn(async move {
                if let Ok(mut tcp) = tokio::net::TcpStream::connect((host.as_str(), port)).await {
                    let mut stream = channel.into_stream();
                    let _ = tokio::io::copy_bidirectional(&mut tcp, &mut stream).await;
                }
            });
        }
        Ok(())
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
    key_id: Option<String>,
    identity_file: Option<String>,
    secret: Option<String>,
    save: bool,
    jumps: Vec<JumpInput>,
    log_name: Option<String>,
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

    // Chain = [bastion-1, …, target]. Each jump's secret comes from the keychain
    // (jumps reference saved hosts); connect the bastion once directly to save it.
    let mut chain: Vec<Hop> = jumps
        .into_iter()
        .map(|j| Hop {
            secret: j
                .host_id
                .as_ref()
                .and_then(|h| crate::secrets::get(h))
                .unwrap_or_default(),
            host: j.host,
            port: j.port,
            user: j.user,
            auth: j.auth,
            key_id: j.key_id,
            identity_file: j.identity_file,
        })
        .collect();
    chain.push(Hop {
        host,
        port,
        user,
        auth,
        key_id,
        identity_file,
        secret,
    });

    let id = format!("s{}", NEXT_ID.fetch_add(1, Ordering::Relaxed));
    state.sessions.lock().unwrap().insert(
        id.clone(),
        Session { tx: None, transport: Transport::Ssh(chain), log_name },
    );
    start_session(app, id.clone(), state.sessions.clone(), state.pending.clone());
    Ok(id)
}

/// Open a raw Telnet session (no auth, no encryption). Uses the same session map
/// and events as SSH, so ssh_write/ssh_resize/ssh_disconnect work uniformly.
#[tauri::command]
pub fn telnet_connect(
    app: AppHandle,
    state: State<'_, SshState>,
    host: String,
    port: u16,
) -> Result<String, String> {
    let id = format!("s{}", NEXT_ID.fetch_add(1, Ordering::Relaxed));
    state.sessions.lock().unwrap().insert(
        id.clone(),
        Session { tx: None, transport: Transport::Telnet(host, port), log_name: None },
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
    let (transport, log_name) = match sessions.lock().unwrap().get(&id) {
        Some(s) => (s.transport.clone(), s.log_name.clone()),
        None => return,
    };
    let (tx, rx) = mpsc::unbounded_channel::<SessionInput>();
    match sessions.lock().unwrap().get_mut(&id) {
        Some(s) => s.tx = Some(tx),
        None => return,
    }

    tauri::async_runtime::spawn(async move {
        let result = match transport {
            Transport::Ssh(chain) => {
                run_session(&app, &id, chain, rx, pending.clone(), log_name).await
            }
            Transport::Telnet(host, port) => crate::telnet::run(&app, &id, host, port, rx).await,
        };
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
/// Authenticate an open handle as `hop` dictates (password or key).
async fn authenticate(handle: &mut client::Handle<Handler>, hop: &Hop) -> Result<(), String> {
    let result = if hop.auth == "key" {
        // Managed key (private material in the keychain) takes precedence; a raw
        // identity-file path is the fallback for keys the manager doesn't own.
        let key = if let Some(kid) = &hop.key_id {
            let pem =
                crate::keys::private_pem(kid).ok_or("managed key not found in keychain")?;
            decode_secret_key(&pem, None).map_err(|e| format!("decode key: {e}"))?
        } else {
            let path = hop.identity_file.as_deref().ok_or("no key selected for key auth")?;
            let passphrase = (!hop.secret.is_empty()).then_some(hop.secret.as_str());
            load_secret_key(expand_tilde(path), passphrase).map_err(|e| format!("load key: {e}"))?
        };
        // RSA keys need an explicit SHA-2 hash alg (rsa-sha2-256); others ignore it.
        let hash = matches!(key.algorithm(), Algorithm::Rsa { .. }).then_some(HashAlg::Sha256);
        handle
            .authenticate_publickey(&hop.user, PrivateKeyWithHashAlg::new(Arc::new(key), hash))
            .await
    } else {
        handle.authenticate_password(&hop.user, &hop.secret).await
    };
    match result.map_err(|e| format!("auth error: {e}"))? {
        client::AuthResult::Success => Ok(()),
        _ => Err("authentication failed".into()),
    }
}

/// Connect + authenticate a hop chain, returning the authenticated target handle
/// plus the bastion handles (kept alive so their transports back the tunnels).
/// `interactive` drives host-key prompting; `forward_target` (remote forwards)
/// and `keepalive` (long-lived forwards) tune the target handler / config.
pub(crate) async fn connect_chain(
    app: &AppHandle,
    id: &str,
    chain: &[Hop],
    pending: Pending,
    interactive: bool,
    forward_target: Option<(String, u16)>,
    keepalive: bool,
) -> Result<(client::Handle<Handler>, Vec<client::Handle<Handler>>), String> {
    let mut cfg = client::Config::default();
    if keepalive {
        cfg.keepalive_interval = Some(std::time::Duration::from_secs(30));
    }
    let config = Arc::new(cfg);
    let n = chain.len();
    let mk = |hop: &Hop, fwd: Option<(String, u16)>| Handler {
        app: app.clone(),
        id: id.to_string(),
        host: hop.host.clone(),
        port: hop.port,
        pending: pending.clone(),
        interactive,
        forward_target: fwd,
    };
    let emit_auth = |hop: &Hop| {
        if interactive {
            let method = if hop.auth == "key" { "publickey" } else { "password" };
            emit_state(app, id, ConnState::Authenticating { method: method.into() });
        }
    };

    // First hop over TCP; each subsequent hop is tunnelled through the previous
    // via a direct-tcpip channel. Only the last (target) hop carries forward_target.
    let first = &chain[0];
    let fwd0 = if n == 1 { forward_target.clone() } else { None };
    let mut handle = client::connect(config.clone(), (first.host.as_str(), first.port), mk(first, fwd0))
        .await
        .map_err(|e| format!("connect failed: {e}"))?;
    emit_auth(first);
    authenticate(&mut handle, first).await?;

    let mut bastions: Vec<client::Handle<Handler>> = Vec::new();
    for (i, hop) in chain.iter().enumerate().skip(1) {
        let channel = handle
            .channel_open_direct_tcpip(hop.host.clone(), hop.port as u32, "127.0.0.1", 0)
            .await
            .map_err(|e| format!("jump to {}: {e}", hop.host))?;
        bastions.push(handle); // keep the previous hop's transport alive
        let fwd = if i == n - 1 { forward_target.clone() } else { None };
        handle = client::connect_stream(config.clone(), channel.into_stream(), mk(hop, fwd))
            .await
            .map_err(|e| format!("connect via jump to {}: {e}", hop.host))?;
        emit_auth(hop);
        authenticate(&mut handle, hop).await?;
    }
    Ok((handle, bastions))
}

/// Build a connect chain for a saved host id: its jump hosts (resolved from the
/// host store) followed by the host itself, each hop's secret pulled from the
/// keychain. Used by background features (forwards) that connect on their own.
pub(crate) fn build_chain(app: &AppHandle, host_id: &str) -> Result<Vec<Hop>, String> {
    let hosts = crate::hosts::read_all(app)?;
    let find = |id: &str| hosts.iter().find(|h| h.id == id);
    let target = find(host_id).ok_or("host not found")?;
    let hop_of = |h: &crate::hosts::Host| Hop {
        host: h.hostname.clone(),
        port: h.port,
        user: h.user.clone(),
        auth: h.auth.clone(),
        key_id: h.key_id.clone(),
        identity_file: h.identity_file.clone(),
        secret: crate::secrets::get(&h.id).unwrap_or_default(),
    };
    let mut chain: Vec<Hop> = target.jumps.iter().filter_map(|jid| find(jid)).map(hop_of).collect();
    chain.push(hop_of(target));
    Ok(chain)
}

async fn run_session(
    app: &AppHandle,
    id: &str,
    chain: Vec<Hop>,
    mut rx: mpsc::UnboundedReceiver<SessionInput>,
    pending: Pending,
    log_name: Option<String>,
) -> Result<Option<String>, String> {
    // Open a session log if the host enabled logging (tee'd in the Data arm below).
    let mut log = log_name.as_deref().and_then(|n| crate::logging::open_log(app, n));
    emit_state(app, id, ConnState::Connecting);
    // `_bastions` must stay in scope for the session's life (they back any jumps).
    let (handle, _bastions) =
        connect_chain(app, id, &chain, pending, true, None, false).await?;

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
                    if let Some(f) = log.as_mut() {
                        use std::io::Write;
                        let _ = f.write_all(&data);
                    }
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
pub(crate) fn expand_tilde(path: &str) -> String {
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
