//! Phase 0 SSH layer: one russh session per connection, owned by an async task.
//! The webview never touches SSH — it invokes commands and subscribes to the
//! `ssh://data` and `ssh://state` events. State events are driven by *real*
//! connection progress so the Phase 1 live stepper is never a faked animation.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use russh::client::{self, KeyboardInteractiveAuthResponse, Prompt};
use russh::keys::{decode_secret_key, load_secret_key, Algorithm, HashAlg, PrivateKeyWithHashAlg};
use russh::{ChannelMsg, MethodKind, MethodSet};
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

/// A parked keyboard-interactive challenge: which connection asked, and where
/// the webview's answer goes. `None` on the channel means the user cancelled.
pub(crate) struct PromptReq {
    conn: String,
    tx: oneshot::Sender<Option<Vec<String>>>,
}

/// Keyboard-interactive challenges awaiting an answer, keyed by a fresh prompt
/// id rather than by connection: unlike the host-key map, several connections
/// can be mid-challenge at once, and per-connection keys would clobber.
pub(crate) type Prompts = Arc<Mutex<HashMap<String, PromptReq>>>;

/// Drops this prompt from the map when the asking task goes away (tab closed,
/// connection aborted), so a dead sender can never linger.
struct PromptGuard {
    prompt_id: String,
    prompts: Prompts,
}

impl Drop for PromptGuard {
    fn drop(&mut self) {
        self.prompts.lock().unwrap().remove(&self.prompt_id);
    }
}

#[derive(Default)]
pub struct SshState {
    // ponytail: single global map guarded by a std Mutex — fine for the handful
    // of sessions a user opens; shard or use dashmap only if that ever bottlenecks.
    sessions: Sessions,
    pending: Pending,
    prompts: Prompts,
}

impl SshState {
    /// Background connectors (forwards, SFTP) share this map so their
    /// keyboard-interactive challenges reach the webview's global dialog.
    pub(crate) fn prompts(&self) -> Prompts {
        self.prompts.clone()
    }
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_PROMPT: AtomicU64 = AtomicU64::new(1);
/// Ceiling on auth round-trips so a server that keeps re-offering the same
/// method can't spin forever.
const MAX_AUTH_ATTEMPTS: usize = 10;
/// How many times a rejected keyboard-interactive challenge may be re-offered.
/// It's the one method whose input changes between attempts, so a mistyped or
/// expired code deserves another go rather than a full reconnect.
const MAX_KBD_ATTEMPTS: usize = 3;

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

/// A single detailed line for the connection-step accordion, emitted on
/// `ssh://log`. `step` buckets the line under one of the UI's fixed steps:
/// "connecting" | "hostkey" | "auth" | "mfa" | "shell". No timestamp — the
/// webview stamps arrival (IPC-local, negligible skew).
#[derive(Serialize, Clone)]
struct LogEvent {
    id: String,
    step: &'static str,
    msg: String,
}

/// One field of a keyboard-interactive challenge. `echo` false means the answer
/// is secret (a password, a code) and the webview must mask it.
#[derive(Serialize, Clone)]
struct PromptField {
    prompt: String,
    echo: bool,
}

/// A keyboard-interactive challenge, emitted on `ssh://prompt`. This is how a
/// verification code is asked for. `conn` is a session id when a pane owns the
/// connection, or a forward/SFTP id when nothing on screen does — the webview
/// renders in place for the former and falls back to a global dialog otherwise.
#[derive(Serialize, Clone)]
struct PromptEvent {
    prompt_id: String,
    conn: String,
    label: String,       // "user@host", so a global dialog can name the asker
    name: String,        // server-supplied title
    instruction: String, // server-supplied instructions (may carry a URL)
    fields: Vec<PromptField>,
}

fn emit_log(app: &AppHandle, id: &str, step: &'static str, msg: impl Into<String>) {
    let _ = app.emit(
        "ssh://log",
        LogEvent { id: id.to_string(), step, msg: msg.into() },
    );
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

/// Surface a failure twice over: the short `Display` form becomes the message
/// the user sees, while the `Debug` form — which names the russh error variant —
/// goes to the connection log, where the detail is actually wanted.
fn fail(
    app: &AppHandle,
    id: &str,
    step: &'static str,
    what: &str,
    e: impl std::fmt::Display + std::fmt::Debug,
) -> String {
    emit_log(app, id, step, format!("{what}: {e:?}"));
    format!("{what}: {e}")
}

/// Wire names for the methods a server still accepts ("keyboard-interactive"),
/// for logs and for the give-up message. russh already maps these.
fn method_list(set: &MethodSet) -> String {
    set.iter()
        .map(|m| <&str>::from(m))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Describe a challenge for the log: prompt text and echo flags only. Answers
/// must never reach a log or an error message (CLAUDE.md).
fn describe(fields: &[Prompt]) -> String {
    fields
        .iter()
        .map(|f| format!("{:?} (echo={})", f.prompt.trim(), f.echo))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Flatten server text to a single line so one log entry stays one row.
fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
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

        emit_log(&self.app, &self.id, "hostkey", format!("{fingerprint} ({key_type})"));
        let existing = crate::knownhosts::lookup(&self.app, &self.host, self.port);
        if let Some(k) = &existing {
            if k.fingerprint == fingerprint {
                emit_log(&self.app, &self.id, "hostkey", "known host, matched");
                return Ok(true); // known and unchanged
            }
        }

        // Non-interactive (forwards): never block on a UI prompt — reject an
        // untrusted key. The user connects the host once to establish trust.
        if !self.interactive {
            emit_log(&self.app, &self.id, "hostkey", "untrusted key, rejected (non-interactive)");
            return Ok(false);
        }
        emit_log(
            &self.app,
            &self.id,
            "hostkey",
            if existing.is_some() { "key CHANGED, awaiting confirmation" } else { "unknown key, awaiting confirmation" },
        );

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
        emit_log(&self.app, &self.id, "hostkey", if accepted { "accepted" } else { "rejected" });
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
    start_session(app, id.clone(), state.sessions.clone(), state.pending.clone(), state.prompts.clone());
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
    start_session(app, id.clone(), state.sessions.clone(), state.pending.clone(), state.prompts.clone());
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
    start_session(app, id, state.sessions.clone(), state.pending.clone(), state.prompts.clone());
    Ok(())
}

/// Fulfill a parked host-key decision from the webview (Accept = true).
#[tauri::command]
pub fn ssh_host_key_decision(state: State<'_, SshState>, id: String, accept: bool) {
    if let Some(tx) = state.pending.lock().unwrap().remove(&id) {
        let _ = tx.send(accept);
    }
}

/// Fulfill a parked keyboard-interactive challenge. `responses` is one answer
/// per field in the order they were sent; `None` means the user cancelled, which
/// aborts authentication. Answers are forwarded to the server and never stored.
#[tauri::command]
pub fn ssh_prompt_response(
    state: State<'_, SshState>,
    prompt_id: String,
    responses: Option<Vec<String>>,
) {
    if let Some(req) = state.prompts.lock().unwrap().remove(&prompt_id) {
        let _ = req.tx.send(responses);
    }
}

/// Spawn the async task that owns the russh session for `id`, wiring a fresh
/// input channel. Reads the retained creds from the map.
fn start_session(app: AppHandle, id: String, sessions: Sessions, pending: Pending, prompts: Prompts) {
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
                run_session(&app, &id, chain, rx, pending.clone(), prompts.clone(), log_name).await
            }
            Transport::Telnet(host, port) => crate::telnet::run(&app, &id, host, port, rx).await,
        };
        pending.lock().unwrap().remove(&id); // clear any un-answered host-key prompt
        prompts.lock().unwrap().retain(|_, r| r.conn != id); // and any un-answered challenge
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

/// Load the hop's private key: a managed key (private material in the keychain)
/// takes precedence; a raw identity-file path is the fallback for keys the
/// manager doesn't own, decrypted with `hop.secret` when it carries a passphrase.
fn load_key(hop: &Hop) -> Result<russh::keys::PrivateKey, String> {
    if let Some(kid) = &hop.key_id {
        let pem = crate::keys::private_pem(kid).ok_or("managed key not found in keychain")?;
        decode_secret_key(&pem, None).map_err(|e| format!("decode key: {e}"))
    } else {
        let path = hop.identity_file.as_deref().ok_or("no key selected for key auth")?;
        let passphrase = (!hop.secret.is_empty()).then_some(hop.secret.as_str());
        load_secret_key(expand_tilde(path), passphrase).map_err(|e| format!("load key: {e}"))
    }
}

/// The next method we can actually serve out of what the server still accepts.
/// Order is deliberate: the host's configured key first, then a stored password,
/// then keyboard-interactive — the one method that can ask the user for a code,
/// so it's the fallback rather than the opener.
fn next_method(
    remaining: &MethodSet,
    tried: &[MethodKind],
    hop: &Hop,
    account_secret: Option<&str>,
) -> Option<MethodKind> {
    let usable = |m: MethodKind| remaining.contains(&m) && !tried.contains(&m);
    if hop.auth == "key" && usable(MethodKind::PublicKey) {
        return Some(MethodKind::PublicKey);
    }
    if account_secret.is_some() && usable(MethodKind::Password) {
        return Some(MethodKind::Password);
    }
    usable(MethodKind::KeyboardInteractive).then_some(MethodKind::KeyboardInteractive)
}

/// Is this challenge just asking for the account password? PAM stacks that use a
/// code as a *second* factor ask for the password first, over the same
/// keyboard-interactive conversation — a single hidden field naming a password.
fn is_password_challenge(fields: &[Prompt]) -> bool {
    let [field] = fields else { return false };
    if field.echo {
        return false;
    }
    let text = field.prompt.to_lowercase();
    text.contains("password") || text.contains("passphrase")
}

fn emit_auth(app: &AppHandle, id: &str, interactive: bool, hop: &Hop, method: MethodKind) {
    let name = <&str>::from(&method);
    if interactive {
        emit_state(app, id, ConnState::Authenticating { method: name.into() });
    }
    emit_log(app, id, "auth", format!("try {name} for {}@{}", hop.user, hop.host));
}

/// Park a channel, emit the challenge, and wait for the webview's answer. A
/// cancel — or a dropped sender, which is what closing the tab looks like —
/// aborts authentication rather than hanging.
async fn ask_prompts(
    app: &AppHandle,
    id: &str,
    label: &str,
    prompts: &Prompts,
    name: &str,
    instruction: &str,
    fields: &[Prompt],
) -> Result<Vec<String>, String> {
    let prompt_id = format!("p{}", NEXT_PROMPT.fetch_add(1, Ordering::Relaxed));
    let (tx, rx) = oneshot::channel::<Option<Vec<String>>>();
    prompts.lock().unwrap().insert(
        prompt_id.clone(),
        PromptReq { conn: id.to_string(), tx },
    );
    let _guard = PromptGuard { prompt_id: prompt_id.clone(), prompts: prompts.clone() };

    let _ = app.emit(
        "ssh://prompt",
        PromptEvent {
            prompt_id,
            conn: id.to_string(),
            label: label.to_string(),
            name: name.to_string(),
            instruction: instruction.to_string(),
            fields: fields
                .iter()
                .map(|f| PromptField { prompt: f.prompt.clone(), echo: f.echo })
                .collect(),
        },
    );
    // ponytail: no timeout, matching the host-key prompt — closing the tab drops
    // the session, which drops the sender and lands in the cancel arm below.
    match rx.await {
        Ok(Some(answers)) => Ok(answers),
        _ => Err("cancelled".into()),
    }
}

/// Walk the server's keyboard-interactive conversation. This is the path a
/// verification code arrives on: the server sends a set of prompts, we answer,
/// and it either accepts, asks again (wrong code), or moves to another factor.
#[allow(clippy::too_many_arguments)]
async fn keyboard_interactive(
    app: &AppHandle,
    id: &str,
    handle: &mut client::Handle<Handler>,
    hop: &Hop,
    prompts: &Prompts,
    label: &str,
    account_secret: Option<&str>,
    secret_spent: &mut bool,
) -> Result<client::AuthResult, String> {
    let mut resp = handle
        .authenticate_keyboard_interactive_start(&hop.user, None)
        .await
        .map_err(|e| fail(app, id, "auth", "keyboard-interactive failed", e))?;

    loop {
        let (name, instruction, fields) = match resp {
            KeyboardInteractiveAuthResponse::Success => return Ok(client::AuthResult::Success),
            KeyboardInteractiveAuthResponse::Failure {
                remaining_methods,
                partial_success,
            } => {
                return Ok(client::AuthResult::Failure {
                    remaining_methods,
                    partial_success,
                })
            }
            KeyboardInteractiveAuthResponse::InfoRequest {
                name,
                instructions,
                prompts,
            } => (name, instructions, prompts),
        };

        let answers = if fields.is_empty() {
            // A request with no fields is the server talking, not asking — PAM
            // banners arrive this way. Answer with nothing rather than showing
            // the user an empty form.
            if !instruction.trim().is_empty() {
                emit_log(app, id, "mfa", format!("server message: {}", one_line(&instruction)));
            }
            Vec::new()
        } else if !*secret_spent && is_password_challenge(&fields) {
            // Answer the password half from what we already hold, so the user is
            // only asked for the part that actually changes — the code. Once
            // only: if the server asks again the stored secret was wrong, and the
            // next pass falls through to prompting.
            match account_secret {
                Some(secret) => {
                    *secret_spent = true;
                    emit_log(app, id, "mfa", "answered password challenge from the saved secret");
                    vec![secret.to_string()]
                }
                None => {
                    emit_log(app, id, "mfa", format!("challenge: {}", describe(&fields)));
                    ask_prompts(app, id, label, prompts, &name, &instruction, &fields).await?
                }
            }
        } else {
            emit_log(app, id, "mfa", format!("challenge: {}", describe(&fields)));
            ask_prompts(app, id, label, prompts, &name, &instruction, &fields).await?
        };

        resp = handle
            .authenticate_keyboard_interactive_respond(answers)
            .await
            .map_err(|e| fail(app, id, "auth", "keyboard-interactive failed", e))?;
    }
}

/// Authenticate one hop, trying every method the server accepts until one lands.
/// This has to be a loop, not a single attempt: a two-factor server answers a
/// *successful* factor with `partial_success` plus the methods still required,
/// which is indistinguishable from a rejection if you only look for `Success`.
async fn authenticate(
    app: &AppHandle,
    id: &str,
    handle: &mut client::Handle<Handler>,
    hop: &Hop,
    prompts: &Prompts,
    interactive: bool,
) -> Result<(), String> {
    let label = format!("{}@{}", hop.user, hop.host);
    // For a key hop `secret` is the key's passphrase, not an account password —
    // sending it as one would hand the server a credential it never asked for.
    let account_secret = (hop.auth != "key" && !hop.secret.is_empty()).then_some(hop.secret.as_str());

    // `none` is what OpenSSH opens with. One packet on the established
    // transport, and the reply lists exactly which methods the server accepts;
    // guessing instead burns a MaxAuthTries slot for every wrong guess.
    let mut remaining = match handle
        .authenticate_none(&hop.user)
        .await
        .map_err(|e| fail(app, id, "auth", "auth error", e))?
    {
        client::AuthResult::Success => {
            emit_log(app, id, "auth", "authenticated (server required no authentication)");
            return Ok(());
        }
        client::AuthResult::Failure { remaining_methods, .. } => remaining_methods,
    };
    emit_log(app, id, "auth", format!("server offers: {}", method_list(&remaining)));

    // Sticky for the whole hop: the stored secret answers at most one password
    // challenge. If the conversation is retried the user is asked for that field
    // too — one extra box, but it's the only way to correct a wrong saved
    // password instead of silently burning every attempt on it.
    let mut secret_spent = false;
    let mut kbd_attempts = 0;
    let mut tried: Vec<MethodKind> = Vec::new();

    for _ in 0..MAX_AUTH_ATTEMPTS {
        let Some(method) = next_method(&remaining, &tried, hop, account_secret) else {
            return Err(format!(
                "authentication failed — server accepts: {}",
                method_list(&remaining)
            ));
        };
        tried.push(method);
        emit_auth(app, id, interactive, hop, method);

        let result = match method {
            MethodKind::PublicKey => {
                let key = load_key(hop)?;
                // RSA keys need an explicit SHA-2 hash alg (rsa-sha2-256); others ignore it.
                let hash = matches!(key.algorithm(), Algorithm::Rsa { .. }).then_some(HashAlg::Sha256);
                handle
                    .authenticate_publickey(&hop.user, PrivateKeyWithHashAlg::new(Arc::new(key), hash))
                    .await
                    .map_err(|e| fail(app, id, "auth", "auth error", e))?
            }
            MethodKind::Password => handle
                .authenticate_password(&hop.user, account_secret.unwrap_or_default())
                .await
                .map_err(|e| fail(app, id, "auth", "auth error", e))?,
            MethodKind::KeyboardInteractive => {
                kbd_attempts += 1;
                keyboard_interactive(
                    app,
                    id,
                    handle,
                    hop,
                    prompts,
                    &label,
                    account_secret,
                    &mut secret_spent,
                )
                .await?
            }
            // next_method never returns the others.
            _ => return Err("authentication failed".into()),
        };

        let name = <&str>::from(&method);
        match result {
            client::AuthResult::Success => {
                emit_log(app, id, "auth", format!("authenticated ({name})"));
                return Ok(());
            }
            client::AuthResult::Failure {
                remaining_methods,
                partial_success,
            } => {
                if partial_success {
                    // The factor was accepted and the server wants another one.
                    // Each factor is its own stage, so what we've already tried
                    // stops disqualifying anything.
                    emit_log(
                        app,
                        id,
                        "auth",
                        format!(
                            "{name} accepted, another factor required: {}",
                            method_list(&remaining_methods)
                        ),
                    );
                    tried.clear();
                } else {
                    emit_log(app, id, "auth", format!("{name} rejected"));
                    // A wrong code shouldn't cost a reconnect. Retrying the
                    // other methods would be pointless — same key, same stored
                    // password, same answer — so only this one is re-offered.
                    if method == MethodKind::KeyboardInteractive && kbd_attempts < MAX_KBD_ATTEMPTS {
                        tried.retain(|m| *m != MethodKind::KeyboardInteractive);
                    }
                }
                remaining = remaining_methods;
            }
        }
    }
    Err("authentication failed — too many attempts".into())
}

/// Connect + authenticate a hop chain, returning the authenticated target handle
/// plus the bastion handles (kept alive so their transports back the tunnels).
/// `interactive` drives host-key prompting; `forward_target` (remote forwards)
/// and `keepalive` (long-lived forwards) tune the target handler / config.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn connect_chain(
    app: &AppHandle,
    id: &str,
    chain: &[Hop],
    pending: Pending,
    prompts: Prompts,
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

    // First hop over TCP; each subsequent hop is tunnelled through the previous
    // via a direct-tcpip channel. Only the last (target) hop carries forward_target.
    let first = &chain[0];
    let fwd0 = if n == 1 { forward_target.clone() } else { None };
    // Resolve up front: russh would do it internally, but doing it here costs
    // nothing extra and lets the log name the addresses actually dialled — and a
    // DNS failure reads as one instead of a generic connect error. The whole
    // list is passed on so multi-address hosts keep their fallback.
    let addrs: Vec<std::net::SocketAddr> = tokio::net::lookup_host((first.host.as_str(), first.port))
        .await
        .map_err(|e| fail(app, id, "connecting", &format!("resolve {}", first.host), e))?
        .collect();
    emit_log(
        app,
        id,
        "connecting",
        format!(
            "resolved {} -> {}",
            first.host,
            addrs.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(", ")
        ),
    );
    emit_log(app, id, "connecting", format!("TCP connect {}:{}", first.host, first.port));
    let mut handle = client::connect(config.clone(), &addrs[..], mk(first, fwd0))
        .await
        .map_err(|e| fail(app, id, "connecting", "connect failed", e))?;
    emit_log(app, id, "connecting", "transport established");
    authenticate(app, id, &mut handle, first, &prompts, interactive).await?;

    let mut bastions: Vec<client::Handle<Handler>> = Vec::new();
    for (i, hop) in chain.iter().enumerate().skip(1) {
        emit_log(app, id, "connecting", format!("via {} -> {}:{}", chain[i - 1].host, hop.host, hop.port));
        let channel = handle
            .channel_open_direct_tcpip(hop.host.clone(), hop.port as u32, "127.0.0.1", 0)
            .await
            .map_err(|e| fail(app, id, "connecting", &format!("jump to {}", hop.host), e))?;
        bastions.push(handle); // keep the previous hop's transport alive
        let fwd = if i == n - 1 { forward_target.clone() } else { None };
        handle = client::connect_stream(config.clone(), channel.into_stream(), mk(hop, fwd))
            .await
            .map_err(|e| fail(app, id, "connecting", &format!("connect via jump to {}", hop.host), e))?;
        emit_log(app, id, "connecting", "transport established");
        authenticate(app, id, &mut handle, hop, &prompts, interactive).await?;
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

/// Owns the russh session for its whole lifetime. Returns an optional
/// disconnect reason on clean exit, or an error string on failure.
#[allow(clippy::too_many_arguments)]
async fn run_session(
    app: &AppHandle,
    id: &str,
    chain: Vec<Hop>,
    mut rx: mpsc::UnboundedReceiver<SessionInput>,
    pending: Pending,
    prompts: Prompts,
    log_name: Option<String>,
) -> Result<Option<String>, String> {
    // Open a session log if the host enabled logging (tee'd in the Data arm below).
    let mut log = log_name.as_deref().and_then(|n| crate::logging::open_log(app, n));
    emit_state(app, id, ConnState::Connecting);
    // `_bastions` must stay in scope for the session's life (they back any jumps).
    let (handle, _bastions) =
        connect_chain(app, id, &chain, pending, prompts, true, None, false).await?;

    emit_log(app, id, "shell", "open session channel");
    let mut channel = handle
        .channel_open_session()
        .await
        .map_err(|e| fail(app, id, "shell", "channel open failed", e))?;

    // Default PTY size; the frontend sends a real size right after connecting.
    emit_log(app, id, "shell", "request pty xterm-256color");
    channel
        .request_pty(false, "xterm-256color", 80, 24, 0, 0, &[])
        .await
        .map_err(|e| fail(app, id, "shell", "pty request failed", e))?;
    emit_log(app, id, "shell", "request shell");
    channel
        .request_shell(false)
        .await
        .map_err(|e| fail(app, id, "shell", "shell request failed", e))?;

    emit_log(app, id, "shell", "session ready");
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

/// Expand a leading `~/` to the home dir. ssh_config IdentityFile paths use it;
/// russh does not. Reads `HOME` (unix) and falls back to `USERPROFILE` (Windows).
/// ponytail: only the leading `~`, which is the case that occurs.
pub(crate) fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
            // On Windows use Path::join so the OS separator is correct, then
            // normalise any remaining forward-slashes in the rest component.
            #[cfg(target_os = "windows")]
            return std::path::Path::new(&home)
                .join(rest)
                .to_string_lossy()
                .replace('/', "\\");

            #[cfg(not(target_os = "windows"))]
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
    use super::*;

    fn hop(auth: &str, secret: &str) -> Hop {
        Hop {
            host: "h".into(),
            port: 22,
            user: "u".into(),
            auth: auth.into(),
            key_id: None,
            identity_file: None,
            secret: secret.into(),
        }
    }

    fn prompt(text: &str, echo: bool) -> Prompt {
        Prompt { prompt: text.into(), echo }
    }

    // The auto-answer shortcut must fire only for a lone hidden password field.
    // Anything else — a code, an echoed field, several fields at once — has to
    // reach the user, or we'd silently send the password as the second factor.
    #[test]
    fn password_challenge_detection() {
        assert!(is_password_challenge(&[prompt("Password: ", false)]));
        assert!(is_password_challenge(&[prompt("Enter passphrase:", false)]));
        assert!(!is_password_challenge(&[prompt("Verification code: ", false)]));
        assert!(!is_password_challenge(&[prompt("Password: ", true)]));
        assert!(!is_password_challenge(&[
            prompt("Password: ", false),
            prompt("Verification code: ", false),
        ]));
        assert!(!is_password_challenge(&[]));
    }

    // Method selection: the configured method first, keyboard-interactive as the
    // fallback, and nothing at all once the server's list is exhausted.
    #[test]
    fn method_selection_order() {
        let all = MethodSet::from(
            &[MethodKind::PublicKey, MethodKind::Password, MethodKind::KeyboardInteractive][..],
        );
        let key_hop = hop("key", "");
        let pw_hop = hop("password", "s3cret");

        assert_eq!(next_method(&all, &[], &key_hop, None), Some(MethodKind::PublicKey));
        assert_eq!(
            next_method(&all, &[], &pw_hop, Some("s3cret")),
            Some(MethodKind::Password)
        );
        // Key rejected -> fall through to the method that can ask for a code.
        assert_eq!(
            next_method(&all, &[MethodKind::PublicKey], &key_hop, None),
            Some(MethodKind::KeyboardInteractive)
        );
        // A password host with nothing saved skips straight to the challenge,
        // rather than sending an empty password and burning a MaxAuthTries slot.
        assert_eq!(
            next_method(&all, &[], &hop("password", ""), None),
            Some(MethodKind::KeyboardInteractive)
        );
        // Server only offers keyboard-interactive: don't try password at all.
        let kbd = MethodSet::from(&[MethodKind::KeyboardInteractive][..]);
        assert_eq!(
            next_method(&kbd, &[], &pw_hop, Some("s3cret")),
            Some(MethodKind::KeyboardInteractive)
        );
        assert_eq!(
            next_method(&kbd, &[MethodKind::KeyboardInteractive], &pw_hop, Some("s3cret")),
            None
        );
    }

    // Wire names, not Debug names — they end up in the log and in the give-up
    // message the user reads.
    #[test]
    fn method_list_uses_wire_names() {
        let set = MethodSet::from(&[MethodKind::PublicKey, MethodKind::KeyboardInteractive][..]);
        assert_eq!(method_list(&set), "publickey, keyboard-interactive");
    }

    // Challenge logging carries prompt text and echo flags only — never answers.
    #[test]
    fn describe_shows_prompts_not_answers() {
        let fields = [prompt("Password: ", false), prompt("Username: ", true)];
        assert_eq!(
            describe(&fields),
            r#""Password:" (echo=false), "Username:" (echo=true)"#
        );
    }


    // One test, not two: `expand_tilde` reads process-global env vars, and Rust
    // runs tests in parallel threads — two tests each mutating HOME would race.
    #[test]
    fn tilde_expansion() {
        // Unix path behaviour — also the macOS CI path.
        #[cfg(not(target_os = "windows"))]
        {
            std::env::set_var("HOME", "/Users/x");
            assert_eq!(expand_tilde("~/.ssh/id_ed25519"), "/Users/x/.ssh/id_ed25519");
            // Absolute and bare-tilde paths pass through untouched.
            assert_eq!(expand_tilde("/etc/ssh/key"), "/etc/ssh/key");
            assert_eq!(expand_tilde("~root/key"), "~root/key");
        }

        // Windows: USERPROFILE replaces HOME; Path::join + separator normalisation
        // must produce all-backslash paths so russh and ssh-key can open them.
        #[cfg(target_os = "windows")]
        {
            std::env::remove_var("HOME");
            std::env::set_var("USERPROFILE", "C:\\Users\\x");
            assert_eq!(expand_tilde("~/.ssh/config"), "C:\\Users\\x\\.ssh\\config");
            assert_eq!(expand_tilde("C:\\absolute\\key"), "C:\\absolute\\key");
            assert_eq!(expand_tilde("~root/key"), "~root/key");
        }
    }
}
