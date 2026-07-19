//! In-app SSH key manager (design-spec §13). Private key material lives only in
//! the OS keychain (`secrets.rs`, account `key:<id>`); `keys.json` holds public
//! metadata (name, type, fingerprint, public key). Generation uses the `ssh-key`
//! crate re-exported by russh — no new dependency. Imported keys are normalized
//! to *decrypted* OpenSSH before storage: the keychain is the vault, so a
//! separate key passphrase would only add a redundant prompt at connect time.
//! FIDO2/hardware keys are a later (paid) tier.

use std::fs;
use std::path::PathBuf;

use russh::keys::ssh_key::getrandom::SysRng;
use russh::keys::ssh_key::rand_core::UnwrapErr;
use russh::keys::ssh_key::LineEnding;
use russh::keys::{decode_secret_key, Algorithm, EcdsaCurve, HashAlg, PrivateKey};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Serialize, Deserialize, Clone)]
pub struct KeyMeta {
    pub id: String,
    pub name: String,
    pub key_type: String,   // Algorithm display, e.g. "ssh-ed25519"
    pub fingerprint: String, // "SHA256:<base64>"
    pub public_key: String, // authorized_keys line
    pub created: String,    // unix seconds
}

fn acct(id: &str) -> String {
    format!("key:{id}")
}

fn store_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("no app data dir: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("create data dir: {e}"))?;
    Ok(dir.join("keys.json"))
}

fn read_all(app: &AppHandle) -> Result<Vec<KeyMeta>, String> {
    let path = store_path(app)?;
    match fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| format!("parse keys.json: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(format!("read keys.json: {e}")),
    }
}

fn write_all(app: &AppHandle, keys: &[KeyMeta]) -> Result<(), String> {
    let path = store_path(app)?;
    let json = serde_json::to_vec_pretty(keys).map_err(|e| format!("serialize keys: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("write keys.json: {e}"))
}

/// Encode + stash the private key in the keychain, record its public metadata.
fn store_key(app: &AppHandle, id: String, name: String, key: PrivateKey) -> Result<KeyMeta, String> {
    let pem = key
        .to_openssh(LineEnding::LF)
        .map_err(|e| format!("encode key: {e}"))?;
    crate::secrets::secret_set(acct(&id), pem.to_string())?;

    let meta = KeyMeta {
        id: id.clone(),
        name,
        key_type: key.algorithm().to_string(),
        fingerprint: key.public_key().fingerprint(HashAlg::Sha256).to_string(),
        public_key: key
            .public_key()
            .to_openssh()
            .map_err(|e| format!("encode public key: {e}"))?,
        created: crate::knownhosts::now_secs(),
    };
    let mut keys = read_all(app)?;
    match keys.iter_mut().find(|k| k.id == id) {
        Some(existing) => *existing = meta.clone(),
        None => keys.push(meta.clone()),
    }
    write_all(app, &keys)?;
    Ok(meta)
}

#[tauri::command]
pub fn keys_list(app: AppHandle) -> Result<Vec<KeyMeta>, String> {
    read_all(&app)
}

/// Generate a new key. `key_type` is "ed25519" | "rsa" | "ecdsa". RSA is 4096,
/// ECDSA is NIST P-256 — the strong defaults; selectors are a later refinement.
fn alg_for(key_type: &str) -> Result<Algorithm, String> {
    match key_type {
        "ed25519" => Ok(Algorithm::Ed25519),
        "rsa" => Ok(Algorithm::Rsa { hash: None }),
        "ecdsa" => Ok(Algorithm::Ecdsa {
            curve: EcdsaCurve::NistP256,
        }),
        other => Err(format!("unsupported key type: {other}")),
    }
}

#[tauri::command]
pub fn key_generate(
    app: AppHandle,
    id: String,
    name: String,
    key_type: String,
) -> Result<KeyMeta, String> {
    let mut rng = UnwrapErr(SysRng);
    let key = PrivateKey::random(&mut rng, alg_for(&key_type)?)
        .map_err(|e| format!("generate key: {e}"))?;
    store_key(&app, id, name, key)
}

/// Import a pasted/loaded private key. If it's passphrase-protected, `passphrase`
/// decrypts it once; we then store it decrypted (see module note).
#[tauri::command]
pub fn key_import(
    app: AppHandle,
    id: String,
    name: String,
    pem: String,
    passphrase: Option<String>,
) -> Result<KeyMeta, String> {
    let pass = passphrase.as_deref().filter(|p| !p.is_empty());
    let key = decode_secret_key(&pem, pass).map_err(|e| format!("parse key: {e}"))?;
    store_key(&app, id, name, key)
}

#[tauri::command]
pub fn key_delete(app: AppHandle, id: String) -> Result<(), String> {
    let mut keys = read_all(&app)?;
    keys.retain(|k| k.id != id);
    crate::secrets::delete(&acct(&id));
    write_all(&app, &keys)
}

/// Fetch a managed private key's OpenSSH text from the keychain (backend-internal;
/// used by the SSH auth path). Not a command — never expose private keys to the UI.
pub fn private_pem(id: &str) -> Option<String> {
    crate::secrets::get(&acct(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alg_mapping_and_generation() {
        assert!(alg_for("bogus").is_err());
        // ed25519 is fast; exercises the SysRng/UnwrapErr wiring end to end.
        let mut rng = UnwrapErr(SysRng);
        let key = PrivateKey::random(&mut rng, alg_for("ed25519").unwrap()).unwrap();
        assert!(key
            .public_key()
            .fingerprint(HashAlg::Sha256)
            .to_string()
            .starts_with("SHA256:"));
        assert!(key.to_openssh(LineEnding::LF).unwrap().contains("OPENSSH"));
    }
}
