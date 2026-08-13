//! Passphrase-encrypted secret store — the Linux fallback for when the session
//! has no D-Bus Secret Service (see `secrets.rs`). GNOME/KDE users never touch
//! this; it exists so a minimal desktop, a tiling WM, or a headless session can
//! still hold keys and passwords instead of failing outright.
//!
//! One file, `secrets.store` in the app data dir, mode 0600: an Argon2id-derived
//! key protecting an AES-256-GCM sealed JSON map of `account -> secret`. The
//! whole map is re-sealed on every write — it holds a handful of entries, so the
//! cost is noise next to the KDF.
//!
//! No new crates: `argon2`, `aes-gcm`, and `zeroize` are already in the tree via
//! `ssh-key`/`ssh-cipher` (encrypted OpenSSH key support), so this only adds
//! direct dependency edges to code the binary already carries.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use russh::keys::ssh_key::getrandom;
use zeroize::Zeroizing;

const FILE_NAME: &str = "secrets.store";

// Magic + format version. The trailing byte is the version; bump it if the
// layout below ever changes so an old file is rejected loudly, not misparsed.
const MAGIC: [u8; 8] = *b"CDTSTOR\x01";

const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

// magic(8) | m_cost(4) | t_cost(4) | p_cost(4) | salt(16) | nonce(12)
const HEADER_LEN: usize = 8 + 4 + 4 + 4 + SALT_LEN + NONCE_LEN;

// Argon2id defaults from the RustCrypto crate: 19 MiB, 2 passes, 1 lane. Costs
// ~50-100ms once per app run, behind a passphrase prompt the user is already
// waiting on — it never touches startup or connect latency. The parameters live
// in the header so they can be raised later without orphaning existing files.
fn default_params() -> Params {
    Params::DEFAULT
}

pub fn path(dir: &Path) -> PathBuf {
    dir.join(FILE_NAME)
}

pub fn exists(dir: &Path) -> bool {
    path(dir).is_file()
}

fn random(buf: &mut [u8]) -> Result<(), String> {
    getrandom::fill(buf).map_err(|e| format!("no system randomness: {e}"))
}

fn derive(passphrase: &str, salt: &[u8], params: Params) -> Result<Zeroizing<[u8; KEY_LEN]>, String> {
    let mut key = Zeroizing::new([0u8; KEY_LEN]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(passphrase.as_bytes(), salt, key.as_mut())
        // The error can name parameter values but never the passphrase; still,
        // keep it generic so nothing about the secret reaches a log.
        .map_err(|_| "could not derive a key from that passphrase".to_string())?;
    Ok(key)
}

/// An unlocked store. Holds the derived key for the process lifetime so a single
/// passphrase prompt covers the whole session — the same lifetime as the
/// plaintext memo cache in `secrets.rs`, so this widens no exposure.
pub struct FileStore {
    path: PathBuf,
    key: Zeroizing<[u8; KEY_LEN]>,
    salt: [u8; SALT_LEN],
    params: Params,
    entries: BTreeMap<String, String>,
}

/// Create a brand-new store. Fails if one is already there — callers must
/// `unlock` instead, so a typo'd passphrase can never silently orphan secrets.
pub fn create(dir: &Path, passphrase: &str) -> Result<FileStore, String> {
    let path = path(dir);
    if path.exists() {
        return Err("a secret store already exists".into());
    }
    let mut salt = [0u8; SALT_LEN];
    random(&mut salt)?;
    let params = default_params();
    let key = derive(passphrase, &salt, params.clone())?;

    let store = FileStore {
        path,
        key,
        salt,
        params,
        entries: BTreeMap::new(),
    };
    store.persist()?;
    Ok(store)
}

/// Open an existing store. A wrong passphrase surfaces as `Err` from the GCM tag
/// check — it can never yield a partial or wrong plaintext.
pub fn unlock(dir: &Path, passphrase: &str) -> Result<FileStore, String> {
    let path = path(dir);
    let raw = fs::read(&path).map_err(|e| format!("read secret store: {e}"))?;
    if raw.len() < HEADER_LEN || raw[..8] != MAGIC {
        return Err("secret store is not a Conduit store, or was written by a newer version".into());
    }

    let header = &raw[..HEADER_LEN];
    let u32_at = |o: usize| u32::from_le_bytes([raw[o], raw[o + 1], raw[o + 2], raw[o + 3]]);
    let params = Params::new(u32_at(8), u32_at(12), u32_at(16), Some(KEY_LEN))
        .map_err(|e| format!("secret store has unusable KDF parameters: {e}"))?;

    let mut salt = [0u8; SALT_LEN];
    salt.copy_from_slice(&raw[20..20 + SALT_LEN]);
    let nonce = &raw[20 + SALT_LEN..HEADER_LEN];

    let nonce = Nonce::try_from(nonce).map_err(|_| "secret store header is truncated".to_string())?;

    let key = derive(passphrase, &salt, params.clone())?;
    let cipher = Aes256Gcm::new_from_slice(key.as_ref())
        .map_err(|_| "internal: bad key length".to_string())?;
    let plain = cipher
        .decrypt(
            &nonce,
            Payload {
                msg: &raw[HEADER_LEN..],
                aad: header,
            },
        )
        .map(Zeroizing::new)
        // A tag failure is overwhelmingly a wrong passphrase; it is also what a
        // tampered or truncated file looks like. Both mean "cannot open".
        .map_err(|_| "incorrect passphrase, or the secret store has been modified".to_string())?;

    let entries: BTreeMap<String, String> =
        serde_json::from_slice(&plain).map_err(|e| format!("parse secret store: {e}"))?;

    Ok(FileStore {
        path,
        key,
        salt,
        params,
        entries,
    })
}

impl FileStore {
    pub fn get(&self, acct: &str) -> Option<String> {
        self.entries.get(acct).cloned()
    }

    pub fn set(&mut self, acct: &str, secret: &str) -> Result<(), String> {
        self.entries.insert(acct.to_string(), secret.to_string());
        self.persist()
    }

    /// Removing an absent account is not an error — matches the keyring path,
    /// where `NoEntry` on delete is treated as success.
    pub fn delete(&mut self, acct: &str) -> Result<(), String> {
        if self.entries.remove(acct).is_none() {
            return Ok(());
        }
        self.persist()
    }

    /// Seal the whole map and swap it into place. A fresh nonce every write (the
    /// salt, and so the key, stay put — re-deriving on each write would pay the
    /// Argon2 cost over and over for no gain).
    fn persist(&self) -> Result<(), String> {
        let mut nonce = [0u8; NONCE_LEN];
        random(&mut nonce)?;

        let mut header = Vec::with_capacity(HEADER_LEN);
        header.extend_from_slice(&MAGIC);
        header.extend_from_slice(&self.params.m_cost().to_le_bytes());
        header.extend_from_slice(&self.params.t_cost().to_le_bytes());
        header.extend_from_slice(&self.params.p_cost().to_le_bytes());
        header.extend_from_slice(&self.salt);
        header.extend_from_slice(&nonce);

        let plain = Zeroizing::new(
            serde_json::to_vec(&self.entries).map_err(|e| format!("serialize secrets: {e}"))?,
        );
        let cipher = Aes256Gcm::new_from_slice(self.key.as_ref())
            .map_err(|_| "internal: bad key length".to_string())?;
        // The header is authenticated but not encrypted, so the version, KDF
        // parameters, salt, and nonce cannot be swapped without failing the tag.
        let sealed = cipher
            .encrypt(
                &Nonce::from(nonce),
                Payload {
                    msg: &plain,
                    aad: &header,
                },
            )
            .map_err(|_| "could not encrypt the secret store".to_string())?;

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("create data dir: {e}"))?;
            // Best-effort: the store itself is 0600 regardless, this just keeps
            // the directory listing private too.
            let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
        }

        // Write-then-rename so an interrupted write can never truncate a store
        // that still holds the user's only copy of a private key.
        let tmp = self.path.with_extension("tmp");
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp)
            .map_err(|e| format!("write secret store: {e}"))?;
        f.write_all(&header).map_err(|e| format!("write secret store: {e}"))?;
        f.write_all(&sealed).map_err(|e| format!("write secret store: {e}"))?;
        f.sync_all().map_err(|e| format!("write secret store: {e}"))?;
        drop(f);

        // `mode` above only applies when the file is created; an existing tmp
        // keeps its old bits, so set them explicitly.
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("secure secret store: {e}"))?;
        fs::rename(&tmp, &self.path).map_err(|e| format!("replace secret store: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Scratch dir under the target dir; no external tempfile dependency.
    fn scratch(tag: &str) -> PathBuf {
        let mut n = [0u8; 8];
        random(&mut n).unwrap();
        let dir = std::env::temp_dir().join(format!("conduit-filestore-{tag}-{}", u64::from_le_bytes(n)));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn round_trip_survives_a_relock() {
        let dir = scratch("roundtrip");
        let mut s = create(&dir, "correct horse").unwrap();
        s.set("key:abc", "PRIVATE KEY BODY").unwrap();
        s.set("host-1", "hunter2").unwrap();
        drop(s);

        let s = unlock(&dir, "correct horse").unwrap();
        assert_eq!(s.get("key:abc").as_deref(), Some("PRIVATE KEY BODY"));
        assert_eq!(s.get("host-1").as_deref(), Some("hunter2"));
        assert_eq!(s.get("nope"), None);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn wrong_passphrase_is_an_error_not_a_wrong_plaintext() {
        let dir = scratch("wrongpass");
        let mut s = create(&dir, "right").unwrap();
        s.set("host-1", "hunter2").unwrap();
        drop(s);

        assert!(unlock(&dir, "wrong").is_err());
        // The real passphrase still works — a failed attempt changes nothing.
        assert_eq!(unlock(&dir, "right").unwrap().get("host-1").as_deref(), Some("hunter2"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn tampering_fails_the_tag() {
        let dir = scratch("tamper");
        let mut s = create(&dir, "pw").unwrap();
        s.set("host-1", "hunter2").unwrap();
        drop(s);

        // Flip a ciphertext bit.
        let p = path(&dir);
        let mut raw = fs::read(&p).unwrap();
        let last = raw.len() - 1;
        raw[last] ^= 0x01;
        fs::write(&p, &raw).unwrap();
        assert!(unlock(&dir, "pw").is_err());

        // And a header bit — the AAD binding must catch this too.
        raw[last] ^= 0x01;
        raw[8] ^= 0x01;
        fs::write(&p, &raw).unwrap();
        assert!(unlock(&dir, "pw").is_err());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn file_is_owner_only_after_create_and_after_rewrite() {
        let dir = scratch("perms");
        let mut s = create(&dir, "pw").unwrap();
        let mode = |p: &Path| fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&path(&dir)), 0o600);
        s.set("host-1", "hunter2").unwrap();
        assert_eq!(mode(&path(&dir)), 0o600);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn delete_removes_the_entry_and_leaves_the_store_readable() {
        let dir = scratch("delete");
        let mut s = create(&dir, "pw").unwrap();
        s.set("a", "1").unwrap();
        s.set("b", "2").unwrap();
        s.delete("a").unwrap();
        s.delete("missing").unwrap(); // absent account is not an error
        drop(s);

        let s = unlock(&dir, "pw").unwrap();
        assert_eq!(s.get("a"), None);
        assert_eq!(s.get("b").as_deref(), Some("2"));
        fs::remove_dir_all(&dir).ok();
    }

    /// A short or foreign file must produce a clean error, never a panic — the
    /// parser has to length-check rather than slice blindly.
    #[test]
    fn short_or_foreign_files_error_instead_of_panicking() {
        let dir = scratch("garbage");
        for junk in [b"".as_slice(), b"CDT".as_slice(), b"not a conduit store at all".as_slice()] {
            fs::write(path(&dir), junk).unwrap();
            assert!(unlock(&dir, "pw").is_err());
        }
        // Right magic, but truncated inside the header.
        let mut short = MAGIC.to_vec();
        short.extend_from_slice(&[0u8; 4]);
        fs::write(path(&dir), &short).unwrap();
        assert!(unlock(&dir, "pw").is_err());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn create_refuses_to_clobber_an_existing_store() {
        let dir = scratch("clobber");
        create(&dir, "pw").unwrap();
        assert!(create(&dir, "other").is_err());
        fs::remove_dir_all(&dir).ok();
    }
}
