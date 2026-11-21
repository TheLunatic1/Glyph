use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use anyhow::{anyhow, Context, Result};
use argon2::Argon2;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use rand::{rngs::SysRng, TryRng};
use scrypt::{scrypt, Params as ScryptParams};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use zeroize::Zeroizing;

const SALT_LEN: usize = 16;
const IV_LEN: usize = 12;
const TAG_LEN: usize = 16;
const KEY_LEN: usize = 32;

const V3_VAULT_PREFIX: &str = "v3:";
const KEYRING_SERVICE: &str = "com.glyph.app";
const KEYRING_USER: &str = "vault_master_key_v3";

fn random_bytes(buf: &mut [u8]) -> Result<()> {
    SysRng
        .try_fill_bytes(buf)
        .map_err(|e| anyhow!("OS RNG failure: {e}"))
}

/// Encrypts a UTF-8 payload using Node.js `cryptoUtil.js`-compatible format:
/// `base64(salt[16] || iv[12] || tag[16] || ciphertext)`
/// KDF: scrypt(N=16384 [log_n=14], r=8, p=1, dkLen=32) + AES-256-GCM.
pub fn encrypt_glyph_export(plaintext: &str, password: &str) -> Result<String> {
    if password.is_empty() {
        return Err(anyhow!("Master password cannot be empty"));
    }

    let mut salt = [0u8; SALT_LEN];
    let mut iv = [0u8; IV_LEN];
    random_bytes(&mut salt)?;
    random_bytes(&mut iv)?;

    let mut derived = Zeroizing::new([0u8; KEY_LEN]);
    let params = ScryptParams::new(14, 8, 1)
        .map_err(|e| anyhow!("Invalid scrypt params: {e}"))?;
    scrypt(password.as_bytes(), &salt, &params, derived.as_mut())
        .map_err(|e| anyhow!("Scrypt key derivation failed: {e}"))?;

    let key = Key::<Aes256Gcm>::from(*derived);
    let cipher = Aes256Gcm::new(&key);
    let nonce = Nonce::from(iv);

    let out = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| anyhow!("AES-256-GCM encryption failed: {e}"))?;
    if out.len() < TAG_LEN {
        return Err(anyhow!("Unexpected ciphertext length"));
    }
    // aes-gcm appends tag at the end; Node cryptoUtil.js expects tag before ciphertext.
    let split_at = out.len() - TAG_LEN;
    let (ciphertext, tag) = out.split_at(split_at);

    let mut combined = Vec::with_capacity(SALT_LEN + IV_LEN + TAG_LEN + ciphertext.len());
    combined.extend_from_slice(&salt);
    combined.extend_from_slice(&iv);
    combined.extend_from_slice(tag);
    combined.extend_from_slice(ciphertext);

    Ok(B64.encode(combined))
}

/// Decrypts a `.glyph` payload created by Glyph Desktop (v2 Node or v3 Rust) or Glyph Mobile.
pub fn decrypt_glyph_export(encrypted_b64: &str, password: &str) -> Result<String> {
    let data = B64
        .decode(encrypted_b64.trim())
        .map_err(|_| anyhow!("Failed to decrypt data. Incorrect password or corrupted file."))?;

    if data.len() < SALT_LEN + IV_LEN + TAG_LEN {
        return Err(anyhow!("Invalid file format"));
    }

    let salt = &data[..SALT_LEN];
    let iv: [u8; IV_LEN] = data[SALT_LEN..SALT_LEN + IV_LEN]
        .try_into()
        .map_err(|_| anyhow!("Invalid IV slice"))?;
    let tag = &data[SALT_LEN + IV_LEN..SALT_LEN + IV_LEN + TAG_LEN];
    let ciphertext = &data[SALT_LEN + IV_LEN + TAG_LEN..];

    let mut derived = Zeroizing::new([0u8; KEY_LEN]);
    let params = ScryptParams::new(14, 8, 1)
        .map_err(|e| anyhow!("Invalid scrypt params: {e}"))?;
    scrypt(password.as_bytes(), salt, &params, derived.as_mut())
        .map_err(|e| anyhow!("Scrypt key derivation failed: {e}"))?;

    let key = Key::<Aes256Gcm>::from(*derived);
    let cipher = Aes256Gcm::new(&key);
    let nonce = Nonce::from(iv);

    // Reassemble ciphertext || tag for aes-gcm crate.
    let mut payload = Vec::with_capacity(ciphertext.len() + TAG_LEN);
    payload.extend_from_slice(ciphertext);
    payload.extend_from_slice(tag);

    let plaintext = cipher
        .decrypt(&nonce, payload.as_slice())
        .map_err(|_| anyhow!("Failed to decrypt data. Incorrect password or corrupted file."))?;

    String::from_utf8(plaintext).context("Decrypted payload is not valid UTF-8")
}

/// Manages local at-rest encryption for saved passwords, secrets, and SSH keys.
#[derive(Clone)]
pub struct VaultCrypto {
    master_key: Zeroizing<[u8; KEY_LEN]>,
    pub keyring_available: bool,
}

impl VaultCrypto {
    /// Initializes vault crypto using the OS keychain when available, falling back
    /// to an Argon2id-wrapped key file in `data_dir` when headless/unavailable.
    pub fn init(data_dir: &Path) -> Result<Self> {
        if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
            match entry.get_password() {
                Ok(b64) => {
                    if let Ok(bytes) = B64.decode(b64.trim()) {
                        if bytes.len() == KEY_LEN {
                            let mut key = Zeroizing::new([0u8; KEY_LEN]);
                            key.copy_from_slice(&bytes);
                            return Ok(Self {
                                master_key: key,
                                keyring_available: true,
                            });
                        }
                    }
                }
                Err(keyring::Error::NoEntry) => {
                    let mut key = Zeroizing::new([0u8; KEY_LEN]);
                    random_bytes(key.as_mut())?;
                    let b64 = Zeroizing::new(B64.encode(*key));
                    if entry.set_password(&b64).is_ok() {
                        // Verify round-trip read because some stub backends accept writes silently
                        if entry.get_password().is_ok() {
                            return Ok(Self {
                                master_key: key,
                                keyring_available: true,
                            });
                        }
                    }
                }
                Err(_) => {}
            }
        }

        // Secure local fallback when OS keychain is unavailable (e.g. headless Linux)
        let key_file = data_dir.join(".vault_key.bin");
        let wrapping_key = derive_machine_wrapping_key(data_dir)?;

        if key_file.exists() {
            let raw = fs::read(&key_file).context("Failed to read fallback vault key file")?;
            let unwrapped = unwrap_with_key(&wrapping_key, &raw)
                .context("Failed to unwrap local vault key")?;
            if unwrapped.len() == KEY_LEN {
                let mut key = Zeroizing::new([0u8; KEY_LEN]);
                key.copy_from_slice(&unwrapped);
                return Ok(Self {
                    master_key: key,
                    keyring_available: false,
                });
            }
        }

        let mut key = Zeroizing::new([0u8; KEY_LEN]);
        random_bytes(key.as_mut())?;
        let wrapped = wrap_with_key(&wrapping_key, &*key)?;
        fs::create_dir_all(data_dir)?;
        write_restricted_file(&key_file, &wrapped)?;

        Ok(Self {
            master_key: key,
            keyring_available: false,
        })
    }

    /// Ephemeral deterministic-free constructor for isolated unit tests.
    pub fn from_raw_key(raw: [u8; KEY_LEN], keyring_available: bool) -> Self {
        Self {
            master_key: Zeroizing::new(raw),
            keyring_available,
        }
    }

    pub fn encrypt_str(&self, plaintext: &str) -> Result<String> {
        let mut salt = [0u8; SALT_LEN];
        let mut iv = [0u8; IV_LEN];
        random_bytes(&mut salt)?;
        random_bytes(&mut iv)?;

        let mut subkey = Zeroizing::new([0u8; KEY_LEN]);
        Argon2::default()
            .hash_password_into(&*self.master_key, &salt, subkey.as_mut())
            .map_err(|e| anyhow!("Argon2id subkey derivation failed: {e}"))?;

        let key = Key::<Aes256Gcm>::from(*subkey);
        let cipher = Aes256Gcm::new(&key);
        let nonce = Nonce::from(iv);
        let ct = cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|e| anyhow!("Vault encryption failed: {e}"))?;

        let mut blob = Vec::with_capacity(SALT_LEN + IV_LEN + ct.len());
        blob.extend_from_slice(&salt);
        blob.extend_from_slice(&iv);
        blob.extend_from_slice(&ct);

        Ok(format!("{V3_VAULT_PREFIX}{}", B64.encode(blob)))
    }

    pub fn decrypt_str(&self, encoded: &str) -> Result<String> {
        let b64 = encoded
            .strip_prefix(V3_VAULT_PREFIX)
            .ok_or_else(|| anyhow!("Unsupported vault cipher prefix"))?;
        let blob = B64.decode(b64.trim()).context("Invalid vault base64")?;
        if blob.len() < SALT_LEN + IV_LEN + TAG_LEN {
            return Err(anyhow!("Truncated vault ciphertext"));
        }

        let salt = &blob[..SALT_LEN];
        let iv: [u8; IV_LEN] = blob[SALT_LEN..SALT_LEN + IV_LEN]
            .try_into()
            .map_err(|_| anyhow!("Invalid vault IV"))?;
        let ct = &blob[SALT_LEN + IV_LEN..];

        let mut subkey = Zeroizing::new([0u8; KEY_LEN]);
        Argon2::default()
            .hash_password_into(&*self.master_key, salt, subkey.as_mut())
            .map_err(|e| anyhow!("Argon2id subkey derivation failed: {e}"))?;

        let key = Key::<Aes256Gcm>::from(*subkey);
        let cipher = Aes256Gcm::new(&key);
        let nonce = Nonce::from(iv);

        let pt = cipher
            .decrypt(&nonce, ct)
            .map_err(|_| anyhow!("Vault decryption failed"))?;
        String::from_utf8(pt).context("Decrypted vault value is not valid UTF-8")
    }
}

fn derive_machine_wrapping_key(data_dir: &Path) -> Result<Zeroizing<[u8; KEY_LEN]>> {
    let mut hasher = Sha256::new();
    hasher.update(b"glyph-v3-local-wrap:");
    hasher.update(data_dir.to_string_lossy().as_bytes());
    if let Ok(user) = std::env::var("USERNAME").or_else(|_| std::env::var("USER")) {
        hasher.update(user.as_bytes());
    }
    if let Ok(host) = std::env::var("COMPUTERNAME").or_else(|_| std::env::var("HOSTNAME")) {
        hasher.update(host.as_bytes());
    }
    let seed = hasher.finalize();

    let mut derived = Zeroizing::new([0u8; KEY_LEN]);
    Argon2::default()
        .hash_password_into(&seed, b"glyph-wrap-salt1", derived.as_mut())
        .map_err(|e| anyhow!("Wrapping KDF failed: {e}"))?;
    Ok(derived)
}

fn wrap_with_key(key_bytes: &[u8; KEY_LEN], plaintext: &[u8]) -> Result<Vec<u8>> {
    let mut iv = [0u8; IV_LEN];
    random_bytes(&mut iv)?;
    let key = Key::<Aes256Gcm>::from(*key_bytes);
    let cipher = Aes256Gcm::new(&key);
    let ct = cipher
        .encrypt(&Nonce::from(iv), plaintext)
        .map_err(|e| anyhow!("Key wrap failed: {e}"))?;
    let mut out = Vec::with_capacity(IV_LEN + ct.len());
    out.extend_from_slice(&iv);
    out.extend_from_slice(&ct);
    Ok(out)
}

fn unwrap_with_key(key_bytes: &[u8; KEY_LEN], blob: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    if blob.len() < IV_LEN + TAG_LEN {
        return Err(anyhow!("Corrupted wrapped key"));
    }
    let iv: [u8; IV_LEN] = blob[..IV_LEN].try_into()?;
    let ct = &blob[IV_LEN..];
    let key = Key::<Aes256Gcm>::from(*key_bytes);
    let cipher = Aes256Gcm::new(&key);
    let pt = cipher
        .decrypt(&Nonce::from(iv), ct)
        .map_err(|_| anyhow!("Key unwrap failed"))?;
    Ok(Zeroizing::new(pt))
}

pub fn write_restricted_file(path: &Path, bytes: &[u8]) -> Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        f.write_all(bytes)?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        fs::write(path, bytes)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyph_export_roundtrip() {
        let payload = r#"[{"id":"1","name":"Prod","host":"10.0.0.1","username":"root","port":22,"password":"secret!"}]"#;
        let encrypted = encrypt_glyph_export(payload, "correct-horse-battery").unwrap();
        let decrypted = decrypt_glyph_export(&encrypted, "correct-horse-battery").unwrap();
        assert_eq!(decrypted, payload);

        // Wrong password must fail cleanly
        assert!(decrypt_glyph_export(&encrypted, "wrong-password").is_err());
    }

    #[test]
    fn vault_crypto_roundtrip() {
        let vc = VaultCrypto::from_raw_key([42u8; KEY_LEN], true);
        let enc = vc.encrypt_str("super-secret-token").unwrap();
        assert!(enc.starts_with(V3_VAULT_PREFIX));
        let dec = vc.decrypt_str(&enc).unwrap();
        assert_eq!(dec, "super-secret-token");
    }

    #[test]
    fn node_crypto_util_cross_compatibility() {
        use std::process::Command;
        let Ok(node_check) = Command::new("node").arg("--version").output() else {
            return;
        };
        if !node_check.status.success() {
            return;
        }

        let payload = r#"[{"id":"abc","name":"Home","host":"192.168.1.50","username":"toha","port":22,"password":"p@ssw0rd!"}]"#;
        let pass = "glyph-master-2026";

        // 1. Encrypt in Rust -> Decrypt with exact Node cryptoUtil.js algorithm
        let rust_enc = encrypt_glyph_export(payload, pass).unwrap();
        let node_script = r#"
const crypto = require('crypto');
const [enc, pass] = [process.argv[1], process.argv[2]];
const data = Buffer.from(enc, 'base64');
const salt = data.subarray(0, 16);
const iv = data.subarray(16, 28);
const tag = data.subarray(28, 44);
const ct = data.subarray(44);
const key = crypto.scryptSync(pass, salt, 32);
const d = crypto.createDecipheriv('aes-256-gcm', key, iv);
d.setAuthTag(tag);
const pt = Buffer.concat([d.update(ct), d.final()]).toString('utf8');
process.stdout.write(pt);
"#;
        let out = Command::new("node")
            .args(["-e", node_script, &rust_enc, pass])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "node decrypt failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(String::from_utf8(out.stdout).unwrap(), payload);

        // 2. Encrypt in Node -> Decrypt in Rust
        let node_enc_script = r#"
const crypto = require('crypto');
const [plain, pass] = [process.argv[1], process.argv[2]];
const salt = crypto.randomBytes(16);
const iv = crypto.randomBytes(12);
const key = crypto.scryptSync(pass, salt, 32);
const c = crypto.createCipheriv('aes-256-gcm', key, iv);
const enc = Buffer.concat([c.update(plain, 'utf8'), c.final()]);
const tag = c.getAuthTag();
process.stdout.write(Buffer.concat([salt, iv, tag, enc]).toString('base64'));
"#;
        let out2 = Command::new("node")
            .args(["-e", node_enc_script, payload, pass])
            .output()
            .unwrap();
        assert!(out2.status.success());
        let node_b64 = String::from_utf8(out2.stdout).unwrap();
        let rust_dec = decrypt_glyph_export(&node_b64, pass).unwrap();
        assert_eq!(rust_dec, payload);
    }
}
