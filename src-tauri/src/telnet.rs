//! Minimal Telnet transport (legacy, unencrypted). Shares the SSH session map
//! and the ssh://data / ssh://state events, so the terminal, resize and
//! disconnect paths are identical. We do just enough IAC option negotiation to
//! get a usable character-mode session and strip protocol bytes from the output.

use tauri::AppHandle;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

use crate::ssh::{emit_connected, emit_connecting, emit_data, SessionInput};

const IAC: u8 = 255;
const WILL: u8 = 251;
const WONT: u8 = 252;
const DO: u8 = 253;
const DONT: u8 = 254;
const SB: u8 = 250;
const SE: u8 = 240;
const OPT_ECHO: u8 = 1;
const OPT_SGA: u8 = 3;

enum Iac {
    Normal,
    Cmd,          // saw IAC
    Opt(u8),      // saw IAC <WILL/WONT/DO/DONT>, awaiting option
    Sub,          // inside subnegotiation, until IAC SE
    SubIac,       // inside subnegotiation, saw IAC
}

/// Feed raw socket bytes through the IAC parser: append display bytes to `out`
/// and any negotiation responses to `reply`.
fn filter(state: &mut Iac, data: &[u8], out: &mut Vec<u8>, reply: &mut Vec<u8>) {
    for &b in data {
        match *state {
            Iac::Normal => {
                if b == IAC {
                    *state = Iac::Cmd;
                } else {
                    out.push(b);
                }
            }
            Iac::Cmd => match b {
                IAC => {
                    out.push(IAC); // escaped 0xFF
                    *state = Iac::Normal;
                }
                WILL | WONT | DO | DONT => *state = Iac::Opt(b),
                SB => *state = Iac::Sub,
                _ => *state = Iac::Normal, // 2-byte command (GA/NOP/…): ignore
            },
            Iac::Opt(cmd) => {
                match cmd {
                    // Server offers an option: accept echo + suppress-go-ahead, refuse the rest.
                    WILL => reply.extend_from_slice(&[
                        IAC,
                        if b == OPT_ECHO || b == OPT_SGA { DO } else { DONT },
                        b,
                    ]),
                    // Server asks us to enable an option: agree to SGA, refuse the rest.
                    DO => reply.extend_from_slice(&[IAC, if b == OPT_SGA { WILL } else { WONT }, b]),
                    _ => {} // WONT/DONT: nothing to do
                }
                *state = Iac::Normal;
            }
            Iac::Sub => {
                if b == IAC {
                    *state = Iac::SubIac;
                }
            }
            Iac::SubIac => {
                *state = if b == SE { Iac::Normal } else { Iac::Sub };
            }
        }
    }
}

pub async fn run(
    app: &AppHandle,
    id: &str,
    host: String,
    port: u16,
    mut rx: mpsc::UnboundedReceiver<SessionInput>,
) -> Result<Option<String>, String> {
    emit_connecting(app, id);
    let mut sock = TcpStream::connect((host.as_str(), port))
        .await
        .map_err(|e| format!("telnet connect failed: {e}"))?;
    emit_connected(app, id);

    let mut state = Iac::Normal;
    let mut buf = [0u8; 8192];
    loop {
        tokio::select! {
            n = sock.read(&mut buf) => match n {
                Ok(0) => return Ok(None), // remote closed
                Ok(n) => {
                    let mut out = Vec::new();
                    let mut reply = Vec::new();
                    filter(&mut state, &buf[..n], &mut out, &mut reply);
                    if !reply.is_empty() {
                        sock.write_all(&reply).await.map_err(|e| format!("write failed: {e}"))?;
                    }
                    if !out.is_empty() {
                        emit_data(app, id, out);
                    }
                }
                Err(e) => return Err(format!("read failed: {e}")),
            },
            input = rx.recv() => match input {
                Some(SessionInput::Data(bytes)) => {
                    sock.write_all(&bytes).await.map_err(|e| format!("write failed: {e}"))?;
                }
                Some(SessionInput::Resize { .. }) => { /* telnet NAWS not negotiated; ignore */ }
                None => return Ok(None), // session dropped from the map
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_iac_and_replies_to_negotiation() {
        // "Hi" + IAC WILL ECHO + IAC DO SGA + "!"
        let data = [b'H', b'i', IAC, WILL, OPT_ECHO, IAC, DO, OPT_SGA, b'!'];
        let mut st = Iac::Normal;
        let mut out = Vec::new();
        let mut reply = Vec::new();
        filter(&mut st, &data, &mut out, &mut reply);
        assert_eq!(out, b"Hi!");
        // Accept echo (DO ECHO) and agree to SGA (WILL SGA).
        assert_eq!(reply, [IAC, DO, OPT_ECHO, IAC, WILL, OPT_SGA]);
    }
}
