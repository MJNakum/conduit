//! PuTTY session import (design-spec §19). PuTTY stores sessions in the Windows
//! registry; users export a `.reg` file to move them. We parse those exports:
//! each `[...\PuTTY\Sessions\<name>]` block becomes a Host (HostName, PortNumber,
//! UserName). Non-SSH sessions (Protocol telnet/raw) are skipped.

use crate::hosts::{blank_host, Host};

/// Decode PuTTY's percent-encoding in a session name (`%20` -> space, etc.).
fn decode(name: &str) -> String {
    let bytes = name.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(&name[i + 1..i + 3], 16) {
                out.push(b as char);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// Strip surrounding quotes from a `.reg` string value.
fn unquote(v: &str) -> String {
    v.trim().trim_matches('"').to_string()
}

pub fn parse(text: &str) -> Vec<Host> {
    let mut hosts: Vec<Host> = Vec::new();
    let mut cur: Option<Host> = None;
    let mut is_ssh = true;

    let flush = |hosts: &mut Vec<Host>, cur: &mut Option<Host>, is_ssh: bool| {
        if let Some(h) = cur.take() {
            if is_ssh && !h.hostname.is_empty() {
                hosts.push(h);
            }
        }
    };

    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix('[') {
            flush(&mut hosts, &mut cur, is_ssh);
            is_ssh = true;
            // A session block: [...\PuTTY\Sessions\<name>]
            if let Some(idx) = rest.find("\\Sessions\\") {
                let name = decode(rest[idx + "\\Sessions\\".len()..].trim_end_matches(']'));
                cur = Some(blank_host(&name, &name));
            } else {
                cur = None; // some other registry key
            }
            continue;
        }
        let Some(h) = cur.as_mut() else { continue };
        let Some((key, val)) = line.split_once('=') else { continue };
        match unquote(key).to_lowercase().as_str() {
            "hostname" => h.hostname = unquote(val),
            "username" => h.user = unquote(val),
            "portnumber" => {
                // dword:0000_0016 (hex)
                if let Some(hex) = val.trim().strip_prefix("dword:") {
                    if let Ok(p) = u16::from_str_radix(hex.trim(), 16) {
                        h.port = p;
                    }
                }
            }
            "protocol" => {
                if unquote(val).to_lowercase() != "ssh" {
                    is_ssh = false;
                }
            }
            _ => {}
        }
    }
    flush(&mut hosts, &mut cur, is_ssh);
    hosts
}

#[tauri::command]
pub fn putty_import(path: String) -> Result<Vec<Host>, String> {
    let path = crate::ssh::expand_tilde(&path);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("read {path}: {e}"))?;
    Ok(parse(&text))
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn parses_ssh_sessions_skips_telnet() {
        let reg = "Windows Registry Editor Version 5.00\r\n\
\r\n\
[HKEY_CURRENT_USER\\Software\\SimonTatham\\PuTTY\\Sessions\\Prod%20DB]\r\n\
\"HostName\"=\"db.example.com\"\r\n\
\"PortNumber\"=dword:00000016\r\n\
\"UserName\"=\"deploy\"\r\n\
\"Protocol\"=\"ssh\"\r\n\
\r\n\
[HKEY_CURRENT_USER\\Software\\SimonTatham\\PuTTY\\Sessions\\OldTelnet]\r\n\
\"HostName\"=\"legacy\"\r\n\
\"Protocol\"=\"telnet\"\r\n";
        let hosts = parse(reg);
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].name, "Prod DB");
        assert_eq!(hosts[0].hostname, "db.example.com");
        assert_eq!(hosts[0].port, 22);
        assert_eq!(hosts[0].user, "deploy");
    }
}
