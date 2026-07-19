//! Hand-rolled `ssh_config` import/export (design-spec §19, the no-lock-in
//! promise). No dependency. We model the common keywords onto `Host` and keep
//! every other option line verbatim in `Host::raw`, so what we import we can
//! export again losslessly.
//!
//! Host id == alias: `ProxyJump <alias>` then maps straight onto `Host::jumps`
//! (which hold host ids), so a jump reference needs no separate resolution pass.

use crate::hosts::Host;

fn blank(alias: &str) -> Host {
    Host {
        id: alias.to_string(),
        name: alias.to_string(),
        hostname: String::new(),
        port: 22,
        user: String::new(),
        protocol: "ssh".into(),
        tags: Vec::new(),
        color: None,
        favorite: false,
        group: None,
        auto_reconnect: false,
        auth: "password".into(),
        key_id: None,
        identity_file: None,
        jumps: Vec::new(),
        raw: Vec::new(),
        scheme: None,
        font: None,
        font_size: None,
        logging: false,
    }
}

/// Split a config line into (keyword, value). ssh_config allows `Key Value` or
/// `Key=Value`; keyword match is case-insensitive.
fn split_kv(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let (k, v) = match line.split_once(['=', ' ', '\t']) {
        Some((k, v)) => (k, v),
        None => return None,
    };
    Some((k.trim().to_lowercase(), v.trim().to_string()))
}

/// Parse `ssh_config` text into concrete saved hosts. Wildcard patterns
/// (`Host *`) are skipped — they're defaults, not a host you connect to.
pub fn parse(text: &str) -> Vec<Host> {
    let mut hosts: Vec<Host> = Vec::new();
    let mut cur: Option<Host> = None;

    for line in text.lines() {
        let Some((key, val)) = split_kv(line) else {
            continue;
        };
        if key == "host" {
            if let Some(h) = cur.take() {
                hosts.push(h);
            }
            // First non-wildcard pattern becomes the alias; skip the block if none.
            let alias = val.split_whitespace().find(|p| !p.contains(['*', '?']));
            cur = alias.map(blank);
            continue;
        }
        let Some(h) = cur.as_mut() else { continue };
        match key.as_str() {
            "hostname" => h.hostname = val,
            "user" => h.user = val,
            "port" => {
                if let Ok(p) = val.parse() {
                    h.port = p;
                }
            }
            "identityfile" => {
                h.identity_file = Some(val);
                h.auth = "key".into();
            }
            "proxyjump" => {
                h.jumps = val
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            // Anything we don't model is preserved verbatim for round-trip.
            _ => h.raw.push(line.trim().to_string()),
        }
    }
    if let Some(h) = cur.take() {
        hosts.push(h);
    }
    hosts
}

/// ssh_config Host patterns can't contain spaces; sanitize a display name to a
/// usable alias.
fn alias_of(name: &str) -> String {
    name.split_whitespace().collect::<Vec<_>>().join("-")
}

/// Render all saved hosts to a standard `ssh_config`. `id_to_name` resolves
/// `jumps` (host ids) back to aliases for `ProxyJump`.
pub fn export(hosts: &[Host]) -> String {
    let name_of = |id: &str| {
        hosts
            .iter()
            .find(|h| h.id == id)
            .map(|h| alias_of(&h.name))
            .unwrap_or_else(|| id.to_string())
    };

    let mut out = String::new();
    for h in hosts {
        out.push_str(&format!("Host {}\n", alias_of(&h.name)));
        if !h.hostname.is_empty() {
            out.push_str(&format!("    HostName {}\n", h.hostname));
        }
        if !h.user.is_empty() {
            out.push_str(&format!("    User {}\n", h.user));
        }
        if h.port != 22 {
            out.push_str(&format!("    Port {}\n", h.port));
        }
        if h.auth == "key" {
            if let Some(idf) = &h.identity_file {
                out.push_str(&format!("    IdentityFile {idf}\n"));
            } else if h.key_id.is_some() {
                out.push_str(&format!(
                    "    # managed key: {} (stored in Keychain, not exportable)\n",
                    h.name
                ));
            }
        }
        if !h.jumps.is_empty() {
            let chain: Vec<String> = h.jumps.iter().map(|id| name_of(id)).collect();
            out.push_str(&format!("    ProxyJump {}\n", chain.join(",")));
        }
        for line in &h.raw {
            out.push_str(&format!("    {line}\n"));
        }
        out.push('\n');
    }
    out
}

// ---- Tauri commands -------------------------------------------------------

/// Parse a config file (default `~/.ssh/config`) into hosts for preview. Does
/// NOT save — the webview previews, deselects, then saves via `host_save`.
#[tauri::command]
pub fn ssh_config_import(path: Option<String>) -> Result<Vec<Host>, String> {
    let path = crate::ssh::expand_tilde(&path.unwrap_or_else(|| "~/.ssh/config".into()));
    let text = std::fs::read_to_string(&path).map_err(|e| format!("read {path}: {e}"))?;
    Ok(parse(&text))
}

#[tauri::command]
pub fn ssh_config_export(app: tauri::AppHandle) -> Result<String, String> {
    Ok(export(&crate::hosts::hosts_list(app)?))
}

#[tauri::command]
pub fn ssh_config_export_write(path: String, text: String) -> Result<(), String> {
    let path = crate::ssh::expand_tilde(&path);
    std::fs::write(&path, text).map_err(|e| format!("write {path}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_fields_and_unknown_options() {
        let cfg = "\
Host bastion
    HostName bastion.example.com
    User admin

Host prod
    HostName 10.0.0.5
    User deploy
    Port 2222
    IdentityFile ~/.ssh/id_ed25519
    ProxyJump bastion
    ForwardAgent yes

Host *
    ServerAliveInterval 60
";
        let hosts = parse(cfg);
        // Wildcard block skipped; two concrete hosts.
        assert_eq!(hosts.len(), 2);
        let prod = hosts.iter().find(|h| h.name == "prod").unwrap();
        assert_eq!(prod.hostname, "10.0.0.5");
        assert_eq!(prod.user, "deploy");
        assert_eq!(prod.port, 2222);
        assert_eq!(prod.auth, "key");
        assert_eq!(prod.identity_file.as_deref(), Some("~/.ssh/id_ed25519"));
        assert_eq!(prod.jumps, vec!["bastion".to_string()]);
        assert_eq!(prod.raw, vec!["ForwardAgent yes".to_string()]);

        // Export then re-parse: fields and the unknown option survive.
        let reparsed = parse(&export(&hosts));
        let prod2 = reparsed.iter().find(|h| h.name == "prod").unwrap();
        assert_eq!(prod2.hostname, prod.hostname);
        assert_eq!(prod2.port, prod.port);
        assert_eq!(prod2.jumps, prod.jumps);
        assert_eq!(prod2.raw, prod.raw);
    }
}
