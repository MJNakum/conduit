//! Saved, toggleable port forwards (design-spec §17): Local (-L), Remote (-R),
//! Dynamic/SOCKS (-D). Configs persist in `forwards.json`; a running forward owns
//! its SSH connection (built non-interactively via `ssh::connect_chain`, so the
//! host must already be trusted) and a listener task. Status is pushed on
//! `forward://state` so the UI toggle reflects reality.

use std::collections::HashMap;
use std::fs;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use russh::client::Handle;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::io::{copy_bidirectional, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::ssh::Handler;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ForwardConfig {
    pub id: String,
    pub name: String,
    pub host_id: String,
    pub kind: String, // "local" | "remote" | "dynamic"
    #[serde(default = "default_bind")]
    pub bind_addr: String,
    pub bind_port: u16,
    #[serde(default)]
    pub dest_host: String,
    #[serde(default)]
    pub dest_port: u16,
}

fn default_bind() -> String {
    "127.0.0.1".into()
}

struct Running {
    // Accept loop for local/dynamic (none for remote — the server listens).
    task: Option<tokio::task::JoinHandle<()>>,
    // Kept alive for the forward's lifetime; dropping closes the SSH connection.
    // russh's Handle isn't Clone, so we share it via Arc (its channel ops take &self).
    handle: Arc<Handle<Handler>>,
    _bastions: Vec<Handle<Handler>>,
    // For remote forwards: the server-side (addr, port) to cancel on stop.
    remote_bind: Option<(String, u16)>,
}

#[derive(Default)]
pub struct ForwardState {
    running: Arc<Mutex<HashMap<String, Running>>>,
}

// ---- config store (mirrors hosts.rs) --------------------------------------

fn store_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("no app data dir: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("create data dir: {e}"))?;
    Ok(dir.join("forwards.json"))
}

fn read_all(app: &AppHandle) -> Result<Vec<ForwardConfig>, String> {
    let path = store_path(app)?;
    match fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| format!("parse forwards.json: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(format!("read forwards.json: {e}")),
    }
}

fn write_all(app: &AppHandle, all: &[ForwardConfig]) -> Result<(), String> {
    let path = store_path(app)?;
    let json = serde_json::to_vec_pretty(all).map_err(|e| format!("serialize: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("write forwards.json: {e}"))
}

#[derive(Serialize, Clone)]
struct FwdEvent {
    id: String,
    state: String,
    message: Option<String>,
}

fn emit(app: &AppHandle, id: &str, state: &str, message: Option<String>) {
    let _ = app.emit(
        "forward://state",
        FwdEvent {
            id: id.to_string(),
            state: state.to_string(),
            message,
        },
    );
}

#[tauri::command]
pub fn forwards_list(app: AppHandle) -> Result<Vec<ForwardConfig>, String> {
    read_all(&app)
}

#[tauri::command]
pub fn forward_save(app: AppHandle, forward: ForwardConfig) -> Result<ForwardConfig, String> {
    let mut all = read_all(&app)?;
    match all.iter_mut().find(|f| f.id == forward.id) {
        Some(existing) => *existing = forward.clone(),
        None => all.push(forward.clone()),
    }
    write_all(&app, &all)?;
    Ok(forward)
}

#[tauri::command]
pub fn forward_delete(
    app: AppHandle,
    state: State<'_, ForwardState>,
    id: String,
) -> Result<(), String> {
    stop_inner(&state, &id);
    let mut all = read_all(&app)?;
    all.retain(|f| f.id != id);
    write_all(&app, &all)
}

// ---- start / stop ---------------------------------------------------------

#[tauri::command]
pub async fn forward_start(
    app: AppHandle,
    state: State<'_, ForwardState>,
    ssh: State<'_, crate::ssh::SshState>,
    id: String,
) -> Result<(), String> {
    let cfg = read_all(&app)?
        .into_iter()
        .find(|f| f.id == id)
        .ok_or("no such forward")?;

    emit(&app, &id, "starting", None);
    let result = start_inner(&app, &state, ssh.prompts(), &cfg).await;
    match &result {
        Ok(()) => emit(&app, &id, "active", None),
        Err(e) => emit(&app, &id, "error", Some(e.clone())),
    }
    result
}

#[tauri::command]
pub async fn forward_stop(
    app: AppHandle,
    state: State<'_, ForwardState>,
    id: String,
) -> Result<(), String> {
    stop_inner(&state, &id);
    emit(&app, &id, "stopped", None);
    Ok(())
}

fn stop_inner(state: &ForwardState, id: &str) {
    if let Some(r) = state.running.lock().unwrap().remove(id) {
        if let Some(t) = &r.task {
            t.abort();
        }
        if let Some((addr, port)) = r.remote_bind.clone() {
            let handle = r.handle.clone();
            tokio::spawn(async move {
                let _ = handle.cancel_tcpip_forward(addr, port as u32).await;
            });
        }
        // Dropping r (and its handles) closes the SSH connection.
    }
}

async fn start_inner(
    app: &AppHandle,
    state: &ForwardState,
    prompts: crate::ssh::Prompts,
    cfg: &ForwardConfig,
) -> Result<(), String> {
    if state.running.lock().unwrap().contains_key(&cfg.id) {
        return Ok(()); // already running
    }
    let chain = crate::ssh::build_chain(app, &cfg.host_id)?;
    let forward_target = (cfg.kind == "remote").then(|| (cfg.dest_host.clone(), cfg.dest_port));
    let (handle, bastions) = crate::ssh::connect_chain(
        app,
        &cfg.id,
        &chain,
        crate::ssh::dummy_pending(),
        // Real prompt map: a host behind a verification code still has to be
        // able to ask, and with no pane to render into the webview falls back
        // to its global dialog. One start = at most one challenge; cancelling
        // fails the forward rather than retrying.
        prompts,
        false, // non-interactive: untrusted host key => reject
        forward_target,
        true, // keepalive for long-lived tunnels
    )
    .await?;
    let handle = Arc::new(handle); // russh Handle isn't Clone; share via Arc

    let bind = format!("{}:{}", cfg.bind_addr, cfg.bind_port);
    let (task, remote_bind) = match cfg.kind.as_str() {
        "local" => {
            let listener = TcpListener::bind(&bind)
                .await
                .map_err(|e| format!("bind {bind}: {e}"))?;
            let h = handle.clone();
            let (dh, dp) = (cfg.dest_host.clone(), cfg.dest_port);
            (Some(tokio::spawn(local_loop(listener, h, dh, dp))), None)
        }
        "dynamic" => {
            let listener = TcpListener::bind(&bind)
                .await
                .map_err(|e| format!("bind {bind}: {e}"))?;
            let h = handle.clone();
            (Some(tokio::spawn(dynamic_loop(listener, h))), None)
        }
        "remote" => {
            handle
                .tcpip_forward(cfg.bind_addr.clone(), cfg.bind_port as u32)
                .await
                .map_err(|e| format!("remote forward request: {e}"))?;
            (None, Some((cfg.bind_addr.clone(), cfg.bind_port)))
        }
        other => return Err(format!("unknown forward kind: {other}")),
    };

    state.running.lock().unwrap().insert(
        cfg.id.clone(),
        Running {
            task,
            handle,
            _bastions: bastions,
            remote_bind,
        },
    );
    Ok(())
}

// Local (-L): accept locally, open a direct-tcpip channel to the fixed target.
async fn local_loop(listener: TcpListener, handle: Arc<Handle<Handler>>, dest_host: String, dest_port: u16) {
    loop {
        let Ok((mut tcp, peer)) = listener.accept().await else {
            break;
        };
        let h = handle.clone();
        let (dh, dp) = (dest_host.clone(), dest_port);
        tokio::spawn(async move {
            if let Ok(ch) = h
                .channel_open_direct_tcpip(dh, dp as u32, peer.ip().to_string(), peer.port() as u32)
                .await
            {
                let mut stream = ch.into_stream();
                let _ = copy_bidirectional(&mut tcp, &mut stream).await;
            }
        });
    }
}

// Dynamic (-D): SOCKS5 proxy; the target comes from each CONNECT request.
async fn dynamic_loop(listener: TcpListener, handle: Arc<Handle<Handler>>) {
    loop {
        let Ok((mut tcp, peer)) = listener.accept().await else {
            break;
        };
        let h = handle.clone();
        tokio::spawn(async move {
            let Ok((host, port)) = socks5_connect(&mut tcp).await else {
                return;
            };
            if let Ok(ch) = h
                .channel_open_direct_tcpip(host, port as u32, peer.ip().to_string(), peer.port() as u32)
                .await
            {
                let mut stream = ch.into_stream();
                let _ = copy_bidirectional(&mut tcp, &mut stream).await;
            }
        });
    }
}

/// Minimal SOCKS5 (no auth, CONNECT): greeting, then read the request and reply
/// success. Returns the requested (host, port).
async fn socks5_connect(tcp: &mut TcpStream) -> Result<(String, u16), String> {
    let mut greet = [0u8; 2];
    tcp.read_exact(&mut greet).await.map_err(|e| e.to_string())?;
    if greet[0] != 5 {
        return Err("not socks5".into());
    }
    let mut methods = vec![0u8; greet[1] as usize];
    tcp.read_exact(&mut methods).await.map_err(|e| e.to_string())?;
    tcp.write_all(&[0x05, 0x00]).await.map_err(|e| e.to_string())?; // no-auth

    let mut head = [0u8; 4];
    tcp.read_exact(&mut head).await.map_err(|e| e.to_string())?;
    let mut buf = head.to_vec();
    match head[3] {
        0x01 => {
            let mut a = [0u8; 4];
            tcp.read_exact(&mut a).await.map_err(|e| e.to_string())?;
            buf.extend_from_slice(&a);
        }
        0x04 => {
            let mut a = [0u8; 16];
            tcp.read_exact(&mut a).await.map_err(|e| e.to_string())?;
            buf.extend_from_slice(&a);
        }
        0x03 => {
            let mut l = [0u8; 1];
            tcp.read_exact(&mut l).await.map_err(|e| e.to_string())?;
            buf.push(l[0]);
            let mut d = vec![0u8; l[0] as usize];
            tcp.read_exact(&mut d).await.map_err(|e| e.to_string())?;
            buf.extend_from_slice(&d);
        }
        _ => return Err("bad address type".into()),
    }
    let mut port = [0u8; 2];
    tcp.read_exact(&mut port).await.map_err(|e| e.to_string())?;
    buf.extend_from_slice(&port);

    let target = parse_connect(&buf)?;
    // Reply: success, bound to 0.0.0.0:0 (clients ignore the bind addr for CONNECT).
    tcp.write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
        .await
        .map_err(|e| e.to_string())?;
    Ok(target)
}

/// Parse a SOCKS5 request (from VER through PORT) into (host, port). Pure, tested.
fn parse_connect(b: &[u8]) -> Result<(String, u16), String> {
    if b.len() < 4 {
        return Err("short request".into());
    }
    if b[0] != 5 {
        return Err("not socks5".into());
    }
    if b[1] != 1 {
        return Err("only CONNECT supported".into());
    }
    let (host, rest) = match b[3] {
        0x01 => {
            if b.len() < 10 {
                return Err("short ipv4 request".into());
            }
            (Ipv4Addr::new(b[4], b[5], b[6], b[7]).to_string(), &b[8..])
        }
        0x04 => {
            if b.len() < 22 {
                return Err("short ipv6 request".into());
            }
            let mut a = [0u8; 16];
            a.copy_from_slice(&b[4..20]);
            (Ipv6Addr::from(a).to_string(), &b[20..])
        }
        0x03 => {
            let len = b[4] as usize;
            if b.len() < 5 + len + 2 {
                return Err("short domain request".into());
            }
            (String::from_utf8_lossy(&b[5..5 + len]).to_string(), &b[5 + len..])
        }
        _ => return Err("bad address type".into()),
    };
    let port = u16::from_be_bytes([rest[0], rest[1]]);
    Ok((host, port))
}

#[cfg(test)]
mod tests {
    use super::parse_connect;

    #[test]
    fn parses_ipv4_and_domain_connect() {
        // VER CMD RSV ATYP=1  1.2.3.4  port 8080
        let v4 = [5, 1, 0, 1, 1, 2, 3, 4, 0x1f, 0x90];
        assert_eq!(parse_connect(&v4).unwrap(), ("1.2.3.4".to_string(), 8080));

        // ATYP=3 domain "ex.com" (len 6) port 443
        let mut dom = vec![5u8, 1, 0, 3, 6];
        dom.extend_from_slice(b"ex.com");
        dom.extend_from_slice(&443u16.to_be_bytes());
        assert_eq!(parse_connect(&dom).unwrap(), ("ex.com".to_string(), 443));

        // Malformed: BIND command (not CONNECT) rejected.
        assert!(parse_connect(&[5, 2, 0, 1, 1, 2, 3, 4, 0, 80]).is_err());
        // Truncated.
        assert!(parse_connect(&[5, 1, 0, 1, 1, 2]).is_err());
    }
}
