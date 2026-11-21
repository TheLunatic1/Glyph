use crate::crypto::{
    decrypt_glyph_export, encrypt_glyph_export, write_restricted_file, VaultCrypto,
};
use crate::models::{
    AppLockConfig, AuthMethod, CommandSnippet, ConnectionConfig, HighlightRule, HostGroup,
    HostKeyStatus, IdentityEntry, IdentityInput, IdentitySummary, KnownHostEntry, SavedTunnelRule,
    SecretEntry, SecretSummary, ServerConfigInput, ServerEntry, ServerSummary, SnippetPackage,
    SshKeyAlgorithm, SshKeyEntry, SshKeySummary, SyncBackend, SyncConfig, SyncMergeStats,
};
use anyhow::{anyhow, Context, Result};
use argon2::{
    password_hash::{phc::PasswordHash, PasswordHasher, PasswordVerifier},
    Argon2,
};
use chrono::Utc;
use directories::ProjectDirs;
use regex::Regex;
use serde::{Deserialize, Serialize};
use ssh_key::{
    getrandom::SysRng, rand_core::UnwrapErr, Algorithm, EcdsaCurve, HashAlg, LineEnding, PrivateKey,
};
use std::collections::{HashMap, HashSet};
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
    #[serde(default)]
    groups: Vec<HostGroup>,
    #[serde(default)]
    identities: Vec<IdentityEntry>,
    #[serde(default)]
    snippet_packages: Vec<SnippetPackage>,
    #[serde(default)]
    highlight_rules: Vec<HighlightRule>,
    #[serde(default)]
    app_lock: AppLockConfig,
    #[serde(default)]
    sync_config: SyncConfig,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FullSyncSnapshot {
    version: u32,
    exported_at: String,
    #[serde(default)]
    servers: Vec<ServerEntry>,
    #[serde(default)]
    groups: Vec<HostGroup>,
    #[serde(default)]
    identities: Vec<IdentityEntry>,
    #[serde(default)]
    ssh_keys: Vec<ExportedSshKey>,
    #[serde(default)]
    snippet_packages: Vec<SnippetPackage>,
    #[serde(default)]
    highlight_rules: Vec<HighlightRule>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportedSshKey {
    id: String,
    name: String,
    algorithm: String,
    fingerprint_sha256: String,
    public_key_openssh: String,
    private_key_pem: String,
    has_passphrase: bool,
    created_at: String,
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
        let mut state: VaultState = if vault_file.exists() {
            let raw = fs::read_to_string(&vault_file)
                .with_context(|| format!("Failed to read {}", vault_file.display()))?;
            serde_json::from_str(&raw).unwrap_or_default()
        } else {
            VaultState::default()
        };

        if state.snippet_packages.is_empty() {
            state.snippet_packages = default_snippet_packages();
        }
        if state.highlight_rules.is_empty() {
            state.highlight_rules = default_highlight_rules();
        }

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

    fn encrypt_opt(&self, plain: Option<String>) -> Result<Option<String>> {
        match plain.filter(|p| !p.is_empty()) {
            Some(p) => Ok(Some(self.crypto.encrypt_str(&p)?)),
            None => Ok(None),
        }
    }

    fn decrypt_opt(&self, enc: Option<&String>) -> Result<Option<String>> {
        match enc {
            Some(e) if !e.is_empty() => Ok(Some(self.crypto.decrypt_str(e)?)),
            _ => Ok(None),
        }
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
        let password = self.encrypt_opt(input.password)?;
        let passphrase = self.encrypt_opt(input.passphrase)?;

        let username = input.username.trim().to_string();
        let has_identity = input.identity_id.is_some() || input.group_id.is_some();
        let resolved_user = if username.is_empty() && !has_identity {
            "root".to_string()
        } else {
            username
        };

        let entry = ServerEntry {
            id: id.clone(),
            name,
            host: input.host.trim().to_string(),
            username: resolved_user,
            port: input.port,
            os: None,
            zerotier: input.zerotier.trim().to_string(),
            private_key_path: input.private_key_path.trim().to_string(),
            password,
            auth_method: input.auth_method,
            ssh_key_id: input.ssh_key_id,
            passphrase,
            proxy_jump_id: input.proxy_jump_id,
            tags: input.tags,
            group_id: input.group_id,
            identity_id: input.identity_id,
            forward_agent: input.forward_agent,
            keepalive_secs: input.keepalive_secs,
            startup_command: input.startup_command.trim().to_string(),
            env_vars: input.env_vars,
            port_knock: input.port_knock,
            saved_tunnels: input.saved_tunnels,
            auto_log: input.auto_log,
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
        let saved_tunnels = if input.saved_tunnels.is_empty() {
            existing.saved_tunnels
        } else {
            input.saved_tunnels
        };

        let username = input.username.trim().to_string();
        let has_identity = input.identity_id.is_some() || input.group_id.is_some();
        let resolved_user = if username.is_empty() && !has_identity {
            "root".to_string()
        } else {
            username
        };

        let updated = ServerEntry {
            id: existing.id,
            name: if input.name.trim().is_empty() {
                input.host.trim().to_string()
            } else {
                input.name.trim().to_string()
            },
            host: input.host.trim().to_string(),
            username: resolved_user,
            port: input.port,
            os: existing.os,
            zerotier: input.zerotier.trim().to_string(),
            private_key_path: input.private_key_path.trim().to_string(),
            password,
            auth_method: input.auth_method,
            ssh_key_id: input.ssh_key_id,
            passphrase,
            proxy_jump_id: input.proxy_jump_id,
            tags: input.tags,
            group_id: input.group_id,
            identity_id: input.identity_id,
            forward_agent: input.forward_agent,
            keepalive_secs: input.keepalive_secs,
            startup_command: input.startup_command.trim().to_string(),
            env_vars: input.env_vars,
            port_knock: input.port_knock,
            saved_tunnels,
            auto_log: input.auto_log,
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

    pub fn add_saved_tunnel(&mut self, server_id: &str, rule: SavedTunnelRule) -> Result<()> {
        let srv = self
            .state
            .servers
            .iter_mut()
            .find(|s| s.id == server_id)
            .ok_or_else(|| anyhow!("Server not found"))?;
        srv.saved_tunnels.push(rule);
        self.save()
    }

    pub fn toggle_saved_tunnel_autostart(
        &mut self,
        server_id: &str,
        rule_id: &str,
    ) -> Result<()> {
        let srv = self
            .state
            .servers
            .iter_mut()
            .find(|s| s.id == server_id)
            .ok_or_else(|| anyhow!("Server not found"))?;
        if let Some(r) = srv.saved_tunnels.iter_mut().find(|r| r.id == rule_id) {
            r.auto_start = !r.auto_start;
            self.save()?;
        }
        Ok(())
    }

    pub fn delete_saved_tunnel(&mut self, server_id: &str, rule_id: &str) -> Result<()> {
        let srv = self
            .state
            .servers
            .iter_mut()
            .find(|s| s.id == server_id)
            .ok_or_else(|| anyhow!("Server not found"))?;
        srv.saved_tunnels.retain(|r| r.id != rule_id);
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

        let ancestors = self.ancestor_groups(server.group_id.as_deref());

        let mut username = server.username.trim().to_string();
        let mut password = self.decrypt_opt(server.password.as_ref())?;
        let mut passphrase = self.decrypt_opt(server.passphrase.as_ref())?;
        let mut ssh_key_id = server.ssh_key_id.clone();
        let mut key_path = (!server.private_key_path.trim().is_empty())
            .then(|| server.private_key_path.trim().to_string());
        let mut auth_method = server.auth_method;
        let mut port = server.port;
        let mut proxy_jump_id = server.proxy_jump_id.clone();

        // Determine candidate identities: server's own identity first, then ancestor group identities
        let mut identity_chain: Vec<&IdentityEntry> = Vec::new();
        if let Some(id_ref) = &server.identity_id {
            if let Some(ident) = self.state.identities.iter().find(|i| &i.id == id_ref) {
                identity_chain.push(ident);
            }
        }
        for grp in &ancestors {
            if port == 0 {
                if let Some(gp) = grp.default_port {
                    if gp > 0 {
                        port = gp;
                    }
                }
            }
            if proxy_jump_id.is_none() {
                proxy_jump_id = grp.proxy_jump_id.clone();
            }
            if let Some(id_ref) = &grp.identity_id {
                if let Some(ident) = self.state.identities.iter().find(|i| &i.id == id_ref) {
                    identity_chain.push(ident);
                }
            }
        }

        for ident in identity_chain {
            if username.is_empty() && !ident.username.trim().is_empty() {
                username = ident.username.trim().to_string();
            }
            if password.is_none() {
                password = self.decrypt_opt(ident.password.as_ref())?;
            }
            if passphrase.is_none() {
                passphrase = self.decrypt_opt(ident.passphrase.as_ref())?;
            }
            if ssh_key_id.is_none() && key_path.is_none() {
                if let Some(kid) = &ident.ssh_key_id {
                    ssh_key_id = Some(kid.clone());
                } else if !ident.private_key_path.trim().is_empty() {
                    key_path = Some(ident.private_key_path.trim().to_string());
                }
                if auth_method == AuthMethod::Password && ident.auth_method != AuthMethod::Password {
                    auth_method = ident.auth_method;
                }
            }
        }

        if username.is_empty() {
            username = "root".to_string();
        }
        if port == 0 {
            port = 22;
        }

        let private_key_pem = match &ssh_key_id {
            Some(kid) => Some(self.get_decrypted_ssh_key(kid)?),
            None => None,
        };
        let proxy_jump = match &proxy_jump_id {
            Some(jump_id) if jump_id != id => {
                Some(Box::new(self.build_connection_config(jump_id, depth + 1)?))
            }
            _ => None,
        };

        Ok(ConnectionConfig {
            host: server.host.clone(),
            port,
            username,
            password,
            private_key_path: key_path,
            private_key_pem,
            passphrase,
            auth_method,
            zerotier: server.zerotier.clone(),
            proxy_jump,
            forward_agent: server.forward_agent,
            keepalive_secs: server.keepalive_secs,
            env_vars: server.env_vars.clone(),
            port_knock: server.port_knock.clone(),
        })
    }

    // ── Host Groups (Nested + Inheritance) ────────────────────────────────────

    pub fn list_groups(&self) -> &[HostGroup] {
        &self.state.groups
    }

    pub fn ancestor_groups(&self, start_group_id: Option<&str>) -> Vec<&HostGroup> {
        let mut chain = Vec::new();
        let mut seen = HashSet::new();
        let mut cur = start_group_id;
        while let Some(gid) = cur {
            if !seen.insert(gid) {
                break;
            }
            let Some(grp) = self.state.groups.iter().find(|g| g.id == gid) else {
                break;
            };
            chain.push(grp);
            cur = grp.parent_id.as_deref();
        }
        chain
    }

    pub fn is_server_in_group_subtree(&self, server: &ServerSummary, target_group_id: &str) -> bool {
        self.ancestor_groups(server.group_id.as_deref())
            .into_iter()
            .any(|g| g.id == target_group_id)
    }

    pub fn add_group(&mut self, mut group: HostGroup) -> Result<String> {
        if group.name.trim().is_empty() {
            return Err(anyhow!("Group name is required"));
        }
        group.id = Uuid::new_v4().to_string();
        group.name = group.name.trim().to_string();
        if group.parent_id.as_deref() == Some(group.id.as_str()) {
            group.parent_id = None;
        }
        let id = group.id.clone();
        self.state.groups.push(group);
        self.save()?;
        Ok(id)
    }

    pub fn edit_group(&mut self, id: &str, updated: HostGroup) -> Result<()> {
        if updated.name.trim().is_empty() {
            return Err(anyhow!("Group name is required"));
        }
        if updated.parent_id.as_deref() == Some(id) {
            return Err(anyhow!("A group cannot be its own parent"));
        }
        if let Some(pid) = &updated.parent_id {
            if self
                .ancestor_groups(Some(pid.as_str()))
                .into_iter()
                .any(|g| g.id == id)
            {
                return Err(anyhow!("Cycle detected in group hierarchy"));
            }
        }
        let slot = self
            .state
            .groups
            .iter_mut()
            .find(|g| g.id == id)
            .ok_or_else(|| anyhow!("Group not found"))?;
        slot.name = updated.name.trim().to_string();
        slot.parent_id = updated.parent_id;
        slot.identity_id = updated.identity_id;
        slot.proxy_jump_id = updated.proxy_jump_id;
        slot.default_port = updated.default_port;
        slot.tags = updated.tags;
        self.save()
    }

    pub fn delete_group(&mut self, id: &str) -> Result<()> {
        let parent_of_deleted = self
            .state
            .groups
            .iter()
            .find(|g| g.id == id)
            .and_then(|g| g.parent_id.clone());
        self.state.groups.retain(|g| g.id != id);
        for g in &mut self.state.groups {
            if g.parent_id.as_deref() == Some(id) {
                g.parent_id = parent_of_deleted.clone();
            }
        }
        for s in &mut self.state.servers {
            if s.group_id.as_deref() == Some(id) {
                s.group_id = None;
            }
        }
        self.save()
    }

    // ── Reusable Identities ───────────────────────────────────────────────────

    pub fn list_identities(&self) -> Vec<IdentitySummary> {
        self.state.identities.iter().map(IdentitySummary::from).collect()
    }

    pub fn add_identity(&mut self, input: IdentityInput) -> Result<String> {
        if input.name.trim().is_empty() {
            return Err(anyhow!("Identity name is required"));
        }
        let id = Uuid::new_v4().to_string();
        let entry = IdentityEntry {
            id: id.clone(),
            name: input.name.trim().to_string(),
            username: input.username.trim().to_string(),
            auth_method: input.auth_method,
            password: self.encrypt_opt(input.password)?,
            ssh_key_id: input.ssh_key_id,
            private_key_path: input.private_key_path.trim().to_string(),
            passphrase: self.encrypt_opt(input.passphrase)?,
        };
        self.state.identities.push(entry);
        self.save()?;
        Ok(id)
    }

    pub fn edit_identity(&mut self, id: &str, input: IdentityInput) -> Result<()> {
        let Some(existing) = self.state.identities.iter().find(|i| i.id == id).cloned() else {
            return Err(anyhow!("Identity not found"));
        };
        if input.name.trim().is_empty() {
            return Err(anyhow!("Identity name is required"));
        }
        let password = match input.password.filter(|p| !p.is_empty()) {
            Some(plain) => Some(self.crypto.encrypt_str(&plain)?),
            None => existing.password,
        };
        let passphrase = match input.passphrase.filter(|p| !p.is_empty()) {
            Some(plain) => Some(self.crypto.encrypt_str(&plain)?),
            None => existing.passphrase,
        };
        if let Some(slot) = self.state.identities.iter_mut().find(|i| i.id == id) {
            slot.name = input.name.trim().to_string();
            slot.username = input.username.trim().to_string();
            slot.auth_method = input.auth_method;
            slot.password = password;
            slot.ssh_key_id = input.ssh_key_id;
            slot.private_key_path = input.private_key_path.trim().to_string();
            slot.passphrase = passphrase;
        }
        self.save()
    }

    pub fn delete_identity(&mut self, id: &str) -> Result<()> {
        self.state.identities.retain(|i| i.id != id);
        for s in &mut self.state.servers {
            if s.identity_id.as_deref() == Some(id) {
                s.identity_id = None;
            }
        }
        for g in &mut self.state.groups {
            if g.identity_id.as_deref() == Some(id) {
                g.identity_id = None;
            }
        }
        self.save()
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
            copy.password = self.decrypt_opt(s.password.as_ref())?;
            copy.passphrase = self.decrypt_opt(s.passphrase.as_ref())?;
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
                group_id: s.group_id,
                identity_id: s.identity_id,
                forward_agent: s.forward_agent,
                keepalive_secs: s.keepalive_secs,
                startup_command: s.startup_command,
                env_vars: s.env_vars,
                port_knock: s.port_knock,
                saved_tunnels: s.saved_tunnels,
                auto_log: s.auto_log,
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
                let plain = self.decrypt_opt(s.value.as_ref())?;
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

    // ── Per-Server Command Snippets & Global Snippet Packages ─────────────────

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
        let id = next_snippet_id(list);
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

    pub fn list_snippet_packages(&self) -> &[SnippetPackage] {
        &self.state.snippet_packages
    }

    pub fn add_snippet_package(&mut self, name: &str, description: &str) -> Result<String> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("Package name is required"));
        }
        let id = Uuid::new_v4().to_string();
        self.state.snippet_packages.push(SnippetPackage {
            id: id.clone(),
            name: trimmed.to_string(),
            description: description.trim().to_string(),
            snippets: Vec::new(),
        });
        self.save()?;
        Ok(id)
    }

    pub fn delete_snippet_package(&mut self, package_id: &str) -> Result<()> {
        self.state.snippet_packages.retain(|p| p.id != package_id);
        self.save()
    }

    pub fn add_package_snippet(&mut self, package_id: &str, name: &str, cmd: &str) -> Result<u64> {
        let pkg = self
            .state
            .snippet_packages
            .iter_mut()
            .find(|p| p.id == package_id)
            .ok_or_else(|| anyhow!("Snippet package not found"))?;
        let id = next_snippet_id(&pkg.snippets);
        pkg.snippets.push(CommandSnippet {
            id,
            name: name.trim().to_string(),
            cmd: cmd.to_string(),
            uses: 0,
        });
        self.save()?;
        Ok(id)
    }

    pub fn remove_package_snippet(&mut self, package_id: &str, snippet_id: u64) -> Result<()> {
        if let Some(pkg) = self
            .state
            .snippet_packages
            .iter_mut()
            .find(|p| p.id == package_id)
        {
            pkg.snippets.retain(|s| s.id != snippet_id);
            self.save()?;
        }
        Ok(())
    }

    // ── Terminal Highlight Rules ──────────────────────────────────────────────

    pub fn list_highlight_rules(&self) -> &[HighlightRule] {
        &self.state.highlight_rules
    }

    pub fn save_highlight_rules(&mut self, rules: Vec<HighlightRule>) -> Result<()> {
        self.state.highlight_rules = rules;
        self.save()
    }

    // ── Optional Master Passcode & Idle Lock ──────────────────────────────────

    pub fn app_lock_config(&self) -> &AppLockConfig {
        &self.state.app_lock
    }

    pub fn set_app_lock_passcode(&mut self, passcode: &str, idle_timeout_mins: u32) -> Result<()> {
        if passcode.len() < 4 {
            return Err(anyhow!("Master passcode must be at least 4 characters"));
        }
        let hash = Argon2::default()
            .hash_password(passcode.as_bytes())
            .map_err(|e| anyhow!("Passcode hashing failed: {e}"))?
            .to_string();
        self.state.app_lock = AppLockConfig {
            enabled: true,
            password_hash: Some(hash),
            idle_timeout_mins,
        };
        self.save()
    }

    pub fn set_app_lock_timeout(&mut self, idle_timeout_mins: u32) -> Result<()> {
        self.state.app_lock.idle_timeout_mins = idle_timeout_mins;
        self.save()
    }

    pub fn disable_app_lock(&mut self) -> Result<()> {
        self.state.app_lock = AppLockConfig::default();
        self.save()
    }

    pub fn verify_app_lock_passcode(&self, passcode: &str) -> bool {
        let Some(raw_hash) = self.state.app_lock.password_hash.as_deref() else {
            return !self.state.app_lock.enabled;
        };
        let Ok(parsed) = PasswordHash::new(raw_hash) else {
            return false;
        };
        Argon2::default()
            .verify_password(passcode.as_bytes(), &parsed)
            .is_ok()
    }

    // ── Zero-Knowledge E2EE Sync ──────────────────────────────────────────────

    pub fn sync_config(&self) -> &SyncConfig {
        &self.state.sync_config
    }

    pub fn save_sync_config(
        &mut self,
        backend: SyncBackend,
        folder_path: String,
        webdav_url: String,
        webdav_username: String,
        webdav_password: Option<String>,
    ) -> Result<()> {
        let encrypted_webdav_password = match webdav_password.filter(|p| !p.is_empty()) {
            Some(plain) => Some(self.crypto.encrypt_str(&plain)?),
            None => self.state.sync_config.encrypted_webdav_password.clone(),
        };
        self.state.sync_config = SyncConfig {
            backend,
            folder_path: folder_path.trim().to_string(),
            webdav_url: webdav_url.trim().to_string(),
            webdav_username: webdav_username.trim().to_string(),
            encrypted_webdav_password,
            last_sync_at: self.state.sync_config.last_sync_at.clone(),
        };
        self.save()
    }

    pub fn decrypt_webdav_password(&self) -> Result<Option<String>> {
        self.decrypt_opt(self.state.sync_config.encrypted_webdav_password.as_ref())
    }

    pub fn mark_synced_at(&mut self, iso_ts: String) -> Result<()> {
        self.state.sync_config.last_sync_at = Some(iso_ts);
        self.save()
    }

    pub fn export_full_sync_snapshot(&self, passphrase: &str) -> Result<String> {
        let mut servers = Vec::with_capacity(self.state.servers.len());
        for s in &self.state.servers {
            let mut copy = s.clone();
            copy.password = self.decrypt_opt(s.password.as_ref())?;
            copy.passphrase = self.decrypt_opt(s.passphrase.as_ref())?;
            servers.push(copy);
        }
        let mut identities = Vec::with_capacity(self.state.identities.len());
        for ident in &self.state.identities {
            let mut copy = ident.clone();
            copy.password = self.decrypt_opt(ident.password.as_ref())?;
            copy.passphrase = self.decrypt_opt(ident.passphrase.as_ref())?;
            identities.push(copy);
        }
        let mut ssh_keys = Vec::with_capacity(self.state.ssh_keys.len());
        for k in &self.state.ssh_keys {
            let pem = self.crypto.decrypt_str(&k.encrypted_private_key)?;
            ssh_keys.push(ExportedSshKey {
                id: k.id.clone(),
                name: k.name.clone(),
                algorithm: k.algorithm.clone(),
                fingerprint_sha256: k.fingerprint_sha256.clone(),
                public_key_openssh: k.public_key_openssh.clone(),
                private_key_pem: pem,
                has_passphrase: k.has_passphrase,
                created_at: k.created_at.clone(),
            });
        }

        let snap = FullSyncSnapshot {
            version: 3,
            exported_at: Utc::now().to_rfc3339(),
            servers,
            groups: self.state.groups.clone(),
            identities,
            ssh_keys,
            snippet_packages: self.state.snippet_packages.clone(),
            highlight_rules: self.state.highlight_rules.clone(),
        };
        let json = serde_json::to_string(&snap)?;
        encrypt_glyph_export(&json, passphrase)
    }

    pub fn merge_sync_snapshot(
        &mut self,
        encrypted_blob: &str,
        passphrase: &str,
    ) -> Result<SyncMergeStats> {
        let plain = decrypt_glyph_export(encrypted_blob, passphrase)?;
        let snap: FullSyncSnapshot = serde_json::from_str(&plain)
            .context("Sync snapshot payload is malformed or incompatible")?;
        let mut stats = SyncMergeStats::default();

        for g in snap.groups {
            if !self.state.groups.iter().any(|eg| eg.id == g.id) {
                self.state.groups.push(g);
                stats.groups_added += 1;
            }
        }
        for k in snap.ssh_keys {
            if !self
                .state
                .ssh_keys
                .iter()
                .any(|ek| ek.id == k.id || ek.fingerprint_sha256 == k.fingerprint_sha256)
            {
                let enc_priv = self.crypto.encrypt_str(&k.private_key_pem)?;
                self.state.ssh_keys.push(SshKeyEntry {
                    id: k.id,
                    name: k.name,
                    algorithm: k.algorithm,
                    fingerprint_sha256: k.fingerprint_sha256,
                    public_key_openssh: k.public_key_openssh,
                    encrypted_private_key: enc_priv,
                    has_passphrase: k.has_passphrase,
                    created_at: k.created_at,
                });
                stats.keys_added += 1;
            }
        }
        for mut ident in snap.identities {
            if !self.state.identities.iter().any(|ei| ei.id == ident.id) {
                ident.password = self.encrypt_opt(ident.password)?;
                ident.passphrase = self.encrypt_opt(ident.passphrase)?;
                self.state.identities.push(ident);
                stats.identities_added += 1;
            }
        }
        for pkg in snap.snippet_packages {
            if !self
                .state
                .snippet_packages
                .iter()
                .any(|ep| ep.id == pkg.id || ep.name.eq_ignore_ascii_case(&pkg.name))
            {
                self.state.snippet_packages.push(pkg);
                stats.packages_added += 1;
            }
        }
        for mut srv in snap.servers {
            let exists = self.state.servers.iter().any(|es| {
                es.id == srv.id
                    || (es.host == srv.host
                        && es.port == srv.port
                        && es.username == srv.username)
            });
            if !exists {
                srv.password = self.encrypt_opt(srv.password)?;
                srv.passphrase = self.encrypt_opt(srv.passphrase)?;
                self.state.servers.push(srv);
                stats.servers_added += 1;
            }
        }
        for rule in snap.highlight_rules {
            if !self.state.highlight_rules.iter().any(|r| r.id == rule.id) {
                self.state.highlight_rules.push(rule);
            }
        }

        self.save()?;
        Ok(stats)
    }
}

/// Extracts unique `{{var_name}}` placeholders in left-to-right appearance order.
pub fn extract_template_vars(cmd: &str) -> Vec<String> {
    let re = Regex::new(r"\{\{\s*([a-zA-Z0-9_.\-]+)\s*\}\}").expect("valid regex");
    let mut vars = Vec::new();
    let mut seen = HashSet::new();
    for cap in re.captures_iter(cmd) {
        if let Some(m) = cap.get(1) {
            let name = m.as_str().to_string();
            if seen.insert(name.clone()) {
                vars.push(name);
            }
        }
    }
    vars
}

/// Replaces `{{var}}` placeholders in `cmd` with the provided map values.
pub fn render_template_cmd(cmd: &str, values: &HashMap<String, String>) -> String {
    let re = Regex::new(r"\{\{\s*([a-zA-Z0-9_.\-]+)\s*\}\}").expect("valid regex");
    re.replace_all(cmd, |caps: &regex::Captures<'_>| {
        let key = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        values.get(key).cloned().unwrap_or_default()
    })
    .into_owned()
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
            cmd: "ss -tulpn".to_string(),
            uses: 0,
        },
    ]
}

fn default_snippet_packages() -> Vec<SnippetPackage> {
    vec![
        SnippetPackage {
            id: "pkg-sysops".to_string(),
            name: "System Ops".to_string(),
            description: "Essential Linux admin diagnostics and systemd commands".to_string(),
            snippets: vec![
                CommandSnippet {
                    id: 101,
                    name: "Restart Service".to_string(),
                    cmd: "sudo systemctl restart {{service}} && systemctl status {{service}} --no-pager".to_string(),
                    uses: 0,
                },
                CommandSnippet {
                    id: 102,
                    name: "Tail Journal Logs".to_string(),
                    cmd: "sudo journalctl -u {{service}} -n {{lines}} --no-pager".to_string(),
                    uses: 0,
                },
                CommandSnippet {
                    id: 103,
                    name: "Disk & Memory Snapshot".to_string(),
                    cmd: "df -h && echo --- && free -h".to_string(),
                    uses: 0,
                },
            ],
        },
        SnippetPackage {
            id: "pkg-docker-net".to_string(),
            name: "Docker & Network".to_string(),
            description: "Containers, sockets, and certificate inspection".to_string(),
            snippets: vec![
                CommandSnippet {
                    id: 201,
                    name: "Docker Container Logs".to_string(),
                    cmd: "docker logs --tail {{lines}} {{container}}".to_string(),
                    uses: 0,
                },
                CommandSnippet {
                    id: 202,
                    name: "Listening Sockets".to_string(),
                    cmd: "ss -tulpn".to_string(),
                    uses: 0,
                },
                CommandSnippet {
                    id: 203,
                    name: "Check TLS Expiry".to_string(),
                    cmd: "echo | openssl s_client -servername {{domain}} -connect {{domain}}:443 2>/dev/null | openssl x509 -noout -dates".to_string(),
                    uses: 0,
                },
            ],
        },
    ]
}

fn default_highlight_rules() -> Vec<HighlightRule> {
    vec![
        HighlightRule {
            id: "hl-error".to_string(),
            name: "Errors & Fatal".to_string(),
            pattern: r"(?i)\b(ERROR|FATAL|CRITICAL|PANIC|FAILED|EXCEPTION)\b".to_string(),
            color_hex: "#f87171".to_string(),
            enabled: true,
        },
        HighlightRule {
            id: "hl-warn".to_string(),
            name: "Warnings".to_string(),
            pattern: r"(?i)\b(WARN|WARNING|DEPRECATED|TIMEOUT|RETRY)\b".to_string(),
            color_hex: "#facc15".to_string(),
            enabled: true,
        },
        HighlightRule {
            id: "hl-ok".to_string(),
            name: "Success / OK".to_string(),
            pattern: r"(?i)\b(OK|SUCCESS|READY|CONNECTED|ACTIVE|RUNNING|PASS)\b".to_string(),
            color_hex: "#4ade80".to_string(),
            enabled: true,
        },
        HighlightRule {
            id: "hl-ip".to_string(),
            name: "IPv4 Addresses".to_string(),
            pattern: r"\b(?:\d{1,3}\.){3}\d{1,3}(?::\d{1,5})?\b".to_string(),
            color_hex: "#22d3ee".to_string(),
            enabled: true,
        },
        HighlightRule {
            id: "hl-url".to_string(),
            name: "URLs".to_string(),
            pattern: r"https?://[^\s\x22'<>]+".to_string(),
            color_hex: "#60a5fa".to_string(),
            enabled: true,
        },
    ]
}

fn next_snippet_id(list: &[CommandSnippet]) -> u64 {
    let now_ms = Utc::now().timestamp_millis().max(1) as u64;
    let max_existing = list.iter().map(|c| c.id).max().unwrap_or(0);
    now_ms.max(max_existing.saturating_add(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::run_e2ee_sync;

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

    #[test]
    fn group_inheritance_and_identity_rotation_propagate() {
        let dir = temp_dir("groups_identities");
        let mut vault = VaultManager::open_for_test(dir.clone(), [11u8; 32]).unwrap();

        let ident_id = vault
            .add_identity(IdentityInput {
                name: "Prod Ops".into(),
                username: "opsadmin".into(),
                auth_method: AuthMethod::Password,
                password: Some("initial-secret".into()),
                ..Default::default()
            })
            .unwrap();

        let root_gid = vault
            .add_group(HostGroup {
                id: String::new(),
                name: "Cloud".into(),
                parent_id: None,
                identity_id: Some(ident_id.clone()),
                proxy_jump_id: None,
                default_port: Some(2222),
                tags: vec!["cloud".into()],
            })
            .unwrap();

        let child_gid = vault
            .add_group(HostGroup {
                id: String::new(),
                name: "Prod".into(),
                parent_id: Some(root_gid),
                identity_id: None,
                proxy_jump_id: None,
                default_port: None,
                tags: vec![],
            })
            .unwrap();

        let srv_id = vault
            .add_server(ServerConfigInput {
                name: "API-01".into(),
                host: "10.0.10.1".into(),
                username: String::new(), // Inherited from Cloud group's Identity
                port: 0,                 // Inherited from Cloud group's default_port
                group_id: Some(child_gid),
                ..Default::default()
            })
            .unwrap();

        let cfg = vault.get_connection_config(&srv_id).unwrap();
        assert_eq!(cfg.username, "opsadmin");
        assert_eq!(cfg.port, 2222);
        assert_eq!(cfg.password.as_deref(), Some("initial-secret"));

        // Rotating the shared Identity must immediately propagate to linked servers
        vault
            .edit_identity(
                &ident_id,
                IdentityInput {
                    name: "Prod Ops".into(),
                    username: "opsadmin".into(),
                    auth_method: AuthMethod::Password,
                    password: Some("rotated-secret-2026".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        let cfg2 = vault.get_connection_config(&srv_id).unwrap();
        assert_eq!(cfg2.password.as_deref(), Some("rotated-secret-2026"));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn snippet_template_variables_and_app_lock_work() {
        let vars = extract_template_vars("docker logs --tail {{lines}} {{container}} {{lines}}");
        assert_eq!(vars, vec!["lines", "container"]);

        let mut map = HashMap::new();
        map.insert("lines".into(), "200".into());
        map.insert("container".into(), "nginx".into());
        let rendered = render_template_cmd("docker logs --tail {{lines}} {{container}}", &map);
        assert_eq!(rendered, "docker logs --tail 200 nginx");

        let dir = temp_dir("app_lock");
        let mut vault = VaultManager::open_for_test(dir.clone(), [12u8; 32]).unwrap();
        assert!(!vault.app_lock_config().enabled);
        vault.set_app_lock_passcode("glyph-pass-42", 15).unwrap();
        assert!(vault.app_lock_config().enabled);
        assert!(vault.verify_app_lock_passcode("glyph-pass-42"));
        assert!(!vault.verify_app_lock_passcode("wrong-pass"));
        vault.disable_app_lock().unwrap();
        assert!(!vault.app_lock_config().enabled);

        let _ = fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn e2ee_folder_sync_merges_two_vaults_losslessly() {
        let dir_a = temp_dir("sync_a");
        let dir_b = temp_dir("sync_b");
        let sync_dir = temp_dir("sync_folder");

        let mut vault_a = VaultManager::open_for_test(dir_a.clone(), [21u8; 32]).unwrap();
        vault_a
            .add_server(ServerConfigInput {
                name: "Alpha".into(),
                host: "10.0.0.10".into(),
                username: "ubuntu".into(),
                password: Some("alpha-pw".into()),
                ..Default::default()
            })
            .unwrap();
        vault_a
            .save_sync_config(
                SyncBackend::LocalFolder,
                sync_dir.display().to_string(),
                String::new(),
                String::new(),
                None,
            )
            .unwrap();

        let out_a = run_e2ee_sync(&mut vault_a, "shared-sync-secret", None)
            .await
            .unwrap();
        assert_eq!(out_a.stats.servers_added, 0);

        let mut vault_b = VaultManager::open_for_test(dir_b.clone(), [22u8; 32]).unwrap();
        vault_b
            .add_server(ServerConfigInput {
                name: "Beta".into(),
                host: "10.0.0.20".into(),
                username: "root".into(),
                password: Some("beta-pw".into()),
                ..Default::default()
            })
            .unwrap();
        vault_b
            .save_sync_config(
                SyncBackend::LocalFolder,
                sync_dir.display().to_string(),
                String::new(),
                String::new(),
                None,
            )
            .unwrap();

        let out_b = run_e2ee_sync(&mut vault_b, "shared-sync-secret", None)
            .await
            .unwrap();
        assert_eq!(out_b.stats.servers_added, 1);
        assert_eq!(vault_b.list_servers().len(), 2);

        let out_a2 = run_e2ee_sync(&mut vault_a, "shared-sync-secret", None)
            .await
            .unwrap();
        assert_eq!(out_a2.stats.servers_added, 1);
        assert_eq!(vault_a.list_servers().len(), 2);

        let _ = fs::remove_dir_all(dir_a);
        let _ = fs::remove_dir_all(dir_b);
        let _ = fs::remove_dir_all(sync_dir);
    }

    #[test]
    fn commands_and_secrets_roundtrip() {
        let dir = temp_dir("commands_secrets");
        let mut vault = VaultManager::open_for_test(dir.clone(), [15u8; 32]).unwrap();
        let srv_id = vault
            .add_server(ServerConfigInput {
                name: "OracleProd".into(),
                host: "10.20.30.40".into(),
                username: "opc".into(),
                password: Some("pw".into()),
                ..Default::default()
            })
            .unwrap();

        // Commands: server starts with default seeded commands; adding two yields base + 2
        let base = vault.list_commands(&srv_id).len();
        let c1 = vault.add_command(&srv_id, "Check Disk", "df -h").unwrap();
        let c2 = vault
            .add_command(&srv_id, "Restart Nginx", "sudo systemctl reload nginx")
            .unwrap();
        assert_ne!(c1, c2);
        vault.increment_command_use(&srv_id, c2).unwrap();
        vault.increment_command_use(&srv_id, c2).unwrap();
        let cmds = vault.list_commands(&srv_id);
        assert_eq!(cmds.len(), base + 2);
        assert_eq!(cmds[0].id, c2);
        assert_eq!(cmds[0].uses, 2);
        vault.remove_command(&srv_id, c1).unwrap();
        assert_eq!(vault.list_commands(&srv_id).len(), base + 1);

        // Secrets: add encrypted, list, decrypt, export/import .glyph bundle, delete
        let sec_id = vault
            .add_secret(&srv_id, "API_TOKEN", "super-secret-value-999")
            .unwrap();
        let listed = vault.list_secrets(&srv_id);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "API_TOKEN");
        assert_eq!(
            vault.get_secret_value(&sec_id).unwrap().as_deref(),
            Some("super-secret-value-999")
        );

        let bundle = vault
            .export_secrets_encrypted(&srv_id, "bundle-pw-123")
            .unwrap();
        vault.delete_secret(&sec_id).unwrap();
        assert!(vault.list_secrets(&srv_id).is_empty());

        let imported = vault
            .import_secrets_encrypted(&srv_id, &bundle, "bundle-pw-123")
            .unwrap();
        assert_eq!(imported, 1);
        let listed_after = vault.list_secrets(&srv_id);
        assert_eq!(listed_after.len(), 1);
        assert_eq!(
            vault
                .get_secret_value(&listed_after[0].id)
                .unwrap()
                .as_deref(),
            Some("super-secret-value-999")
        );

        let _ = fs::remove_dir_all(dir);
    }
}
