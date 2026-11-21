use crate::crypto::{decrypt_glyph_export, encrypt_glyph_export, write_restricted_file, VaultCrypto};
use crate::models::{
    AuthMethod, CommandSnippet, ConnectionConfig, HostKeyStatus, KnownHostEntry, SecretEntry,
    SecretSummary, ServerConfigInput, ServerEntry, ServerSummary, SshKeyAlgorithm, SshKeyEntry,
    SshKeySummary,
};
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use ssh_key::{
    getrandom::SysRng, rand_core::UnwrapErr, Algorithm, EcdsaCurve, HashAlg, LineEnding, PrivateKey,
};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use zeroize::Zeroizing;

#[derive(Debug, Default, Serialize, Deserialize)]
struct VaultState {
    #[serde(default)]
    servers: Vec<ServerEntry>,
    #[serde(default)]
    secrets: HashMap<String, Vec<SecretEntry>>,
    #[serde(default)]
    commands: HashMap<String, Vec<CommandSnippet>>,
    #[serde(default)]
    ssh_keys: Vec<SshKeyEntry>,
    #[serde(default)]
    known_hosts: Vec<KnownHostEntry>,
}

pub struct VaultManager {
    data_dir: PathBuf,
    vault_file: PathBuf,
    crypto: VaultCrypto,
    state: VaultState,
}

impl VaultManager {
    pub fn open_default() -> Result<Self> {
        let proj = ProjectDirs::from("com", "glyph", "Glyph")
            .ok_or_else(|| anyhow!("Unable to determine OS application data directory"))?;
        let data_dir = proj.data_local_dir().to_path_buf();
        fs::create_dir_all(&data_dir)?;
        let crypto = VaultCrypto::init(&data_dir)?;
        Self::open_with_crypto(data_dir, crypto)
    }

    pub fn open_for_test(data_dir: PathBuf, raw_key: [u8; 32]) -> Result<Self> {
        fs::create_dir_all(&data_dir)?;
        let crypto = VaultCrypto::from_raw_key(raw_key, true);
        Self::open_with_crypto(data_dir, crypto)
    }

    fn open_with_crypto(data_dir: PathBuf, crypto: VaultCrypto) -> Result<Self> {
        let vault_file = data_dir.join("glyph_vault_v3.json");
        let state = if vault_file.exists() {
            let raw = fs::read_to_string(&vault_file)
                .with_context(|| format!("Failed to read {}", vault_file.display()))?;
            serde_json::from_str(&raw).unwrap_or_default()
        } else {
            VaultState::default()
        };

        Ok(Self {
            data_dir,
            vault_file,
            crypto,
            state,
        })
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn is_encryption_available(&self) -> bool {
        self.crypto.keyring_available
    }

    fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.state)?;
        write_restricted_file(&self.vault_file, json.as_bytes())
    }

    // ── Servers ───────────────────────────────────────────────────────────────

    pub fn list_servers(&self) -> Vec<ServerSummary> {
        self.state.servers.iter().map(ServerSummary::from).collect()
    }

    pub fn get_server(&self, id: &str) -> Option<ServerSummary> {
        self.state
            .servers
            .iter()
            .find(|s| s.id == id)
            .map(ServerSummary::from)
    }

    pub fn add_server(&mut self, input: ServerConfigInput) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let name = if input.name.trim().is_empty() {
            input.host.trim().to_string()
        } else {
            input.name.trim().to_string()
        };
        let password = match input.password.filter(|p| !p.is_empty()) {
            Some(plain) => Some(self.crypto.encrypt_str(&plain)?),
            None => None,
        };
        let passphrase = match input.passphrase.filter(|p| !p.is_empty()) {
            Some(plain) => Some(self.crypto.encrypt_str(&plain)?),
            None => None,
        };

        let entry = ServerEntry {
            id: id.clone(),
            name,
            host: input.host.trim().to_string(),
            username: if input.username.trim().is_empty() {
                "root".to_string()
            } else {
                input.username.trim().to_string()
            },
            port: if input.port == 0 { 22 } else { input.port },
            os: None,
            zerotier: input.zerotier.trim().to_string(),
            private_key_path: input.private_key_path.trim().to_string(),
            password,
            auth_method: input.auth_method,
            ssh_key_id: input.ssh_key_id,
            passphrase,
            proxy_jump_id: input.proxy_jump_id,
            tags: input.tags,
        };

        self.state.servers.push(entry);
        self.save()?;
        Ok(id)
    }

    pub fn edit_server(&mut self, id: &str, input: ServerConfigInput) -> Result<()> {
        let Some(existing) = self.state.servers.iter().find(|s| s.id == id).cloned() else {
            return Err(anyhow!("Server not found"));
        };

        let password = match input.password.filter(|p| !p.is_empty()) {
            Some(plain) => Some(self.crypto.encrypt_str(&plain)?),
            None => existing.password,
        };
        let passphrase = match input.passphrase.filter(|p| !p.is_empty()) {
            Some(plain) => Some(self.crypto.encrypt_str(&plain)?),
            None => existing.passphrase,
        };

        let updated = ServerEntry {
            id: existing.id,
            name: if input.name.trim().is_empty() {
                input.host.trim().to_string()
            } else {
                input.name.trim().to_string()
            },
            host: input.host.trim().to_string(),
            username: if input.username.trim().is_empty() {
                "root".to_string()
            } else {
                input.username.trim().to_string()
            },
            port: if input.port == 0 { 22 } else { input.port },
            os: existing.os,
            zerotier: input.zerotier.trim().to_string(),
            private_key_path: input.private_key_path.trim().to_string(),
            password,
            auth_method: input.auth_method,
            ssh_key_id: input.ssh_key_id,
            passphrase,
            proxy_jump_id: input.proxy_jump_id,
            tags: input.tags,
        };

        if let Some(slot) = self.state.servers.iter_mut().find(|s| s.id == id) {
            *slot = updated;
        }
        self.save()
    }

    pub fn update_server_os(&mut self, id: &str, os: Option<String>) -> Result<()> {
        if let Some(s) = self.state.servers.iter_mut().find(|s| s.id == id) {
            s.os = os;
            self.save()?;
        }
        Ok(())
    }

    pub fn set_server_vault_key(&mut self, server_id: &str, key_id: &str) -> Result<()> {
        if let Some(s) = self.state.servers.iter_mut().find(|s| s.id == server_id) {
            s.auth_method = AuthMethod::VaultKey;
            s.ssh_key_id = Some(key_id.to_string());
            self.save()?;
        }
        Ok(())
    }

    pub fn delete_server(&mut self, id: &str) -> Result<()> {
        self.state.servers.retain(|s| s.id != id);
        self.state.secrets.remove(id);
        self.state.commands.remove(id);
        self.save()
    }

    pub fn get_connection_config(&self, id: &str) -> Result<ConnectionConfig> {
        self.build_connection_config(id, 0)
    }

    fn build_connection_config(&self, id: &str, depth: u8) -> Result<ConnectionConfig> {
        if depth > 4 {
            return Err(anyhow!("ProxyJump chain exceeds maximum depth of 4"));
        }
        let server = self
            .state
            .servers
            .iter()
            .find(|s| s.id == id)
            .ok_or_else(|| anyhow!("Server not found in Vault"))?;

        let password = match &server.password {
            Some(enc) => Some(self.crypto.decrypt_str(enc)?),
            None => None,
        };
        let passphrase = match &server.passphrase {
            Some(enc) => Some(self.crypto.decrypt_str(enc)?),
            None => None,
        };
        let private_key_pem = match &server.ssh_key_id {
            Some(key_id) => Some(self.get_decrypted_ssh_key(key_id)?),
            None => None,
        };
        let proxy_jump = match &server.proxy_jump_id {
            Some(jump_id) if jump_id != id => {
                Some(Box::new(self.build_connection_config(jump_id, depth + 1)?))
            }
            _ => None,
        };

        Ok(ConnectionConfig {
            host: server.host.clone(),
            port: server.port,
            username: server.username.clone(),
            password,
            private_key_path: if server.private_key_path.is_empty() {
                None
            } else {
                Some(server.private_key_path.clone())
            },
            private_key_pem,
            passphrase,
            auth_method: server.auth_method,
            zerotier: server.zerotier.clone(),
            proxy_jump,
        })
    }

    // ── .glyph Backup Export / Import (100% v2 & Mobile Interop) ─────────────

    pub fn export_servers_encrypted(
        &self,
        master_password: &str,
        server_ids: Option<&[String]>,
    ) -> Result<String> {
        let mut export_list: Vec<ServerEntry> = Vec::new();
        for s in &self.state.servers {
            if let Some(ids) = server_ids {
                if !ids.is_empty() && !ids.contains(&s.id) {
                    continue;
                }
            }
            let mut copy = s.clone();
            if let Some(enc) = &s.password {
                copy.password = Some(self.crypto.decrypt_str(enc)?);
            }
            if let Some(enc) = &s.passphrase {
                copy.passphrase = Some(self.crypto.decrypt_str(enc)?);
            }
            export_list.push(copy);
        }
        let json = serde_json::to_string(&export_list)?;
        encrypt_glyph_export(&json, master_password)
    }

    pub fn decode_import_servers(
        &self,
        encrypted_content: &str,
        master_password: &str,
    ) -> Result<Vec<ServerEntry>> {
        let decrypted = decrypt_glyph_export(encrypted_content, master_password)?;
        let list: Vec<ServerEntry> = serde_json::from_str(&decrypted)
            .map_err(|_| anyhow!("Invalid servers list format inside backup file"))?;
        Ok(list)
    }

    pub fn import_servers(&mut self, list: Vec<ServerEntry>) -> Result<usize> {
        let mut added = 0;
        for s in list {
            let input = ServerConfigInput {
                id: None,
                name: s.name,
                host: s.host,
                username: s.username,
                port: s.port,
                password: s.password,
                zerotier: s.zerotier,
                private_key_path: s.private_key_path,
                auth_method: s.auth_method,
                ssh_key_id: s.ssh_key_id,
                passphrase: s.passphrase,
                proxy_jump_id: s.proxy_jump_id,
                tags: s.tags,
            };
            self.add_server(input)?;
            added += 1;
        }
        Ok(added)
    }

    // ── SSH Key Vault ─────────────────────────────────────────────────────────

    pub fn list_ssh_keys(&self) -> Vec<SshKeySummary> {
        self.state.ssh_keys.iter().map(SshKeySummary::from).collect()
    }

    pub fn generate_ssh_key(
        &mut self,
        name: &str,
        algo: SshKeyAlgorithm,
        passphrase: Option<&str>,
    ) -> Result<SshKeySummary> {
        let mut rng = UnwrapErr(SysRng);
        let ssh_alg = match algo {
            SshKeyAlgorithm::Ed25519 => Algorithm::Ed25519,
            SshKeyAlgorithm::Rsa4096 => Algorithm::Rsa { hash: None },
            SshKeyAlgorithm::EcdsaP256 => Algorithm::Ecdsa {
                curve: EcdsaCurve::NistP256,
            },
        };
        let mut private = PrivateKey::random(&mut rng, ssh_alg)
            .map_err(|e| anyhow!("SSH key generation failed: {e}"))?;
        private.set_comment(name);

        let public_key_openssh = private
            .public_key()
            .to_openssh()
            .map_err(|e| anyhow!("Public key encoding failed: {e}"))?;
        let fingerprint_sha256 = private
            .public_key()
            .fingerprint(HashAlg::Sha256)
            .to_string();

        let (pem_str, has_passphrase) = match passphrase.filter(|p| !p.is_empty()) {
            Some(pass) => {
                let enc = private
                    .encrypt(&mut SysRng, pass)
                    .map_err(|e| anyhow!("Private key encryption failed: {e}"))?;
                (
                    Zeroizing::new(
                        enc.to_openssh(LineEnding::LF)
                            .map_err(|e| anyhow!("PEM encode failed: {e}"))?
                            .to_string(),
                    ),
                    true,
                )
            }
            None => (
                Zeroizing::new(
                    private
                        .to_openssh(LineEnding::LF)
                        .map_err(|e| anyhow!("PEM encode failed: {e}"))?
                        .to_string(),
                ),
                false,
            ),
        };

        let encrypted_private_key = self.crypto.encrypt_str(&pem_str)?;
        let entry = SshKeyEntry {
            id: Uuid::new_v4().to_string(),
            name: name.trim().to_string(),
            algorithm: algo.to_string(),
            fingerprint_sha256,
            public_key_openssh,
            encrypted_private_key,
            has_passphrase,
            created_at: Utc::now().to_rfc3339(),
        };
        let summary = SshKeySummary::from(&entry);
        self.state.ssh_keys.push(entry);
        self.save()?;
        Ok(summary)
    }

    pub fn import_ssh_key(
        &mut self,
        name: &str,
        private_key_pem: &str,
        passphrase: Option<&str>,
    ) -> Result<SshKeySummary> {
        let mut parsed = PrivateKey::from_openssh(private_key_pem.trim())
            .map_err(|e| anyhow!("Invalid OpenSSH private key: {e}"))?;
        let has_passphrase = parsed.is_encrypted();
        if has_passphrase {
            let pass = passphrase
                .filter(|p| !p.is_empty())
                .ok_or_else(|| anyhow!("This SSH key is encrypted — passphrase required"))?;
            parsed = parsed
                .decrypt(pass)
                .map_err(|_| anyhow!("Incorrect passphrase for SSH private key"))?;
        }

        let public_key_openssh = parsed
            .public_key()
            .to_openssh()
            .map_err(|e| anyhow!("Public key encoding failed: {e}"))?;
        let fingerprint_sha256 = parsed
            .public_key()
            .fingerprint(HashAlg::Sha256)
            .to_string();
        let algorithm = parsed.algorithm().to_string();

        let encrypted_private_key = self.crypto.encrypt_str(private_key_pem.trim())?;
        let entry = SshKeyEntry {
            id: Uuid::new_v4().to_string(),
            name: name.trim().to_string(),
            algorithm,
            fingerprint_sha256,
            public_key_openssh,
            encrypted_private_key,
            has_passphrase,
            created_at: Utc::now().to_rfc3339(),
        };
        let summary = SshKeySummary::from(&entry);
        self.state.ssh_keys.push(entry);
        self.save()?;
        Ok(summary)
    }

    pub fn delete_ssh_key(&mut self, id: &str) -> Result<()> {
        self.state.ssh_keys.retain(|k| k.id != id);
        for s in &mut self.state.servers {
            if s.ssh_key_id.as_deref() == Some(id) {
                s.ssh_key_id = None;
                if s.auth_method == AuthMethod::VaultKey {
                    s.auth_method = AuthMethod::Password;
                }
            }
        }
        self.save()
    }

    pub fn get_decrypted_ssh_key(&self, id: &str) -> Result<String> {
        let entry = self
            .state
            .ssh_keys
            .iter()
            .find(|k| k.id == id)
            .ok_or_else(|| anyhow!("SSH key not found in Vault"))?;
        self.crypto.decrypt_str(&entry.encrypted_private_key)
    }

    pub fn get_ssh_key_public(&self, id: &str) -> Option<String> {
        self.state
            .ssh_keys
            .iter()
            .find(|k| k.id == id)
            .map(|k| k.public_key_openssh.clone())
    }

    // ── Known Hosts (TOFU) ────────────────────────────────────────────────────

    pub fn verify_host_key(
        &self,
        host: &str,
        port: u16,
        fingerprint_sha256: &str,
    ) -> HostKeyStatus {
        let key = format!("{host}:{port}");
        match self.state.known_hosts.iter().find(|h| h.host_port == key) {
            None => HostKeyStatus::Unknown,
            Some(entry) if entry.fingerprint_sha256 == fingerprint_sha256 => {
                HostKeyStatus::Trusted
            }
            Some(entry) => HostKeyStatus::Mismatch {
                expected: entry.fingerprint_sha256.clone(),
                actual: fingerprint_sha256.to_string(),
            },
        }
    }

    pub fn trust_host_key(
        &mut self,
        host: &str,
        port: u16,
        algorithm: &str,
        fingerprint_sha256: &str,
    ) -> Result<()> {
        let key = format!("{host}:{port}");
        self.state.known_hosts.retain(|h| h.host_port != key);
        self.state.known_hosts.push(KnownHostEntry {
            host_port: key,
            algorithm: algorithm.to_string(),
            fingerprint_sha256: fingerprint_sha256.to_string(),
            added_at: Utc::now().to_rfc3339(),
        });
        self.save()
    }

    pub fn known_hosts_map(&self) -> HashMap<String, String> {
        self.state
            .known_hosts
            .iter()
            .map(|h| (h.host_port.clone(), h.fingerprint_sha256.clone()))
            .collect()
    }

    pub fn list_known_hosts(&self) -> &[KnownHostEntry] {
        &self.state.known_hosts
    }

    pub fn remove_known_host(&mut self, host_port: &str) -> Result<()> {
        let before = self.state.known_hosts.len();
        self.state.known_hosts.retain(|h| h.host_port != host_port);
        if self.state.known_hosts.len() != before {
            self.save()?;
        }
        Ok(())
    }

    // ── Secrets Vault ─────────────────────────────────────────────────────────

    pub fn list_secrets(&self, server_id: &str) -> Vec<SecretSummary> {
        self.state
            .secrets
            .get(server_id)
            .map(|items| {
                items
                    .iter()
                    .map(|s| SecretSummary {
                        id: s.id.clone(),
                        name: s.name.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn add_secret(&mut self, server_id: &str, name: &str, value: &str) -> Result<String> {
        let enc = self.crypto.encrypt_str(value)?;
        let id = Uuid::new_v4().to_string();
        self.state
            .secrets
            .entry(server_id.to_string())
            .or_default()
            .push(SecretEntry {
                id: id.clone(),
                name: name.to_string(),
                value: Some(enc),
            });
        self.save()?;
        Ok(id)
    }

    pub fn delete_secret(&mut self, secret_id: &str) -> Result<()> {
        let mut changed = false;
        for items in self.state.secrets.values_mut() {
            let before = items.len();
            items.retain(|s| s.id != secret_id);
            if items.len() != before {
                changed = true;
            }
        }
        if changed {
            self.save()?;
        }
        Ok(())
    }

    pub fn get_secret_value(&self, secret_id: &str) -> Result<Option<String>> {
        for items in self.state.secrets.values() {
            if let Some(secret) = items.iter().find(|s| s.id == secret_id) {
                if let Some(enc) = &secret.value {
                    return Ok(Some(self.crypto.decrypt_str(enc)?));
                }
            }
        }
        Ok(None)
    }

    pub fn export_secrets_encrypted(
        &self,
        server_id: &str,
        master_password: &str,
    ) -> Result<String> {
        let mut list = Vec::new();
        if let Some(items) = self.state.secrets.get(server_id) {
            for s in items {
                let plain = match &s.value {
                    Some(enc) => Some(self.crypto.decrypt_str(enc)?),
                    None => None,
                };
                list.push(SecretEntry {
                    id: s.id.clone(),
                    name: s.name.clone(),
                    value: plain,
                });
            }
        }
        let json = serde_json::to_string(&list)?;
        encrypt_glyph_export(&json, master_password)
    }

    pub fn import_secrets_encrypted(
        &mut self,
        server_id: &str,
        encrypted_content: &str,
        master_password: &str,
    ) -> Result<usize> {
        let decrypted = decrypt_glyph_export(encrypted_content, master_password)?;
        let list: Vec<SecretEntry> = serde_json::from_str(&decrypted)?;
        let mut count = 0;
        for s in list {
            if let Some(val) = s.value {
                self.add_secret(server_id, &s.name, &val)?;
                count += 1;
            }
        }
        Ok(count)
    }

    // ── Command Snippets ──────────────────────────────────────────────────────

    pub fn list_commands(&self, server_id: &str) -> Vec<CommandSnippet> {
        if let Some(list) = self.state.commands.get(server_id) {
            let mut sorted = list.clone();
            sorted.sort_by_key(|a| std::cmp::Reverse(a.uses));
            return sorted;
        }
        default_commands()
    }

    pub fn add_command(&mut self, server_id: &str, name: &str, cmd: &str) -> Result<u64> {
        let list = self
            .state
            .commands
            .entry(server_id.to_string())
            .or_insert_with(default_commands);
        let id = Utc::now().timestamp_millis() as u64;
        list.push(CommandSnippet {
            id,
            name: name.to_string(),
            cmd: cmd.to_string(),
            uses: 0,
        });
        self.save()?;
        Ok(id)
    }

    pub fn remove_command(&mut self, server_id: &str, id: u64) -> Result<()> {
        let list = self
            .state
            .commands
            .entry(server_id.to_string())
            .or_insert_with(default_commands);
        list.retain(|c| c.id != id);
        self.save()
    }

    pub fn increment_command_use(&mut self, server_id: &str, id: u64) -> Result<()> {
        let list = self
            .state
            .commands
            .entry(server_id.to_string())
            .or_insert_with(default_commands);
        if let Some(c) = list.iter_mut().find(|c| c.id == id) {
            c.uses += 1;
            self.save()?;
        }
        Ok(())
    }
}

fn default_commands() -> Vec<CommandSnippet> {
    vec![
        CommandSnippet {
            id: 1,
            name: "Update System".to_string(),
            cmd: "sudo apt update && sudo apt upgrade -y".to_string(),
            uses: 0,
        },
        CommandSnippet {
            id: 2,
            name: "Check Logs".to_string(),
            cmd: "tail -f /var/log/syslog".to_string(),
            uses: 0,
        },
        CommandSnippet {
            id: 3,
            name: "List Ports".to_string(),
            cmd: "netstat -tulpn".to_string(),
            uses: 0,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("glyph_test_{}_{}", name, Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn ssh_key_keygen_and_import_roundtrip() {
        let dir = temp_dir("ssh_keys");
        let mut vault = VaultManager::open_for_test(dir.clone(), [7u8; 32]).unwrap();

        let summary = vault
            .generate_ssh_key("prod-ed25519", SshKeyAlgorithm::Ed25519, Some("pass123"))
            .unwrap();
        assert!(summary.public_key_openssh.starts_with("ssh-ed25519 "));
        assert!(summary.fingerprint_sha256.starts_with("SHA256:"));
        assert!(summary.has_passphrase);

        let pem = vault.get_decrypted_ssh_key(&summary.id).unwrap();
        assert!(pem.contains("BEGIN OPENSSH PRIVATE KEY"));

        let imported = vault
            .import_ssh_key("imported-copy", &pem, Some("pass123"))
            .unwrap();
        assert_eq!(imported.fingerprint_sha256, summary.fingerprint_sha256);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn server_export_import_roundtrip() {
        let dir = temp_dir("export_import");
        let mut vault = VaultManager::open_for_test(dir.clone(), [9u8; 32]).unwrap();
        let id = vault
            .add_server(ServerConfigInput {
                name: "Home".into(),
                host: "10.55.200.1".into(),
                username: "toha".into(),
                port: 22,
                password: Some("secret-pw".into()),
                ..Default::default()
            })
            .unwrap();

        let blob = vault
            .export_servers_encrypted("backup-pass", Some(&[id]))
            .unwrap();
        let decoded = vault.decode_import_servers(&blob, "backup-pass").unwrap();
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].name, "Home");
        assert_eq!(decoded[0].password.as_deref(), Some("secret-pw"));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn known_hosts_tofu_lifecycle() {
        let dir = temp_dir("known_hosts");
        let mut vault = VaultManager::open_for_test(dir.clone(), [3u8; 32]).unwrap();
        assert_eq!(
            vault.verify_host_key("10.0.0.2", 22, "SHA256:abc"),
            HostKeyStatus::Unknown
        );
        vault
            .trust_host_key("10.0.0.2", 22, "ssh-ed25519", "SHA256:abc")
            .unwrap();
        assert_eq!(
            vault.verify_host_key("10.0.0.2", 22, "SHA256:abc"),
            HostKeyStatus::Trusted
        );
        assert_eq!(
            vault.verify_host_key("10.0.0.2", 22, "SHA256:evil"),
            HostKeyStatus::Mismatch {
                expected: "SHA256:abc".into(),
                actual: "SHA256:evil".into(),
            }
        );

        let _ = fs::remove_dir_all(dir);
    }
}
