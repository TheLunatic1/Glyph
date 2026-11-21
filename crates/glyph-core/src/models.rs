use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TunnelDirection {
    #[default]
    Local,
    Remote,
    Dynamic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum KnockProtocol {
    #[default]
    Tcp,
    Udp,
}

impl std::fmt::Display for KnockProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tcp => write!(f, "tcp"),
            Self::Udp => write!(f, "udp"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortKnockStep {
    #[serde(default)]
    pub protocol: KnockProtocol,
    pub port: u16,
    #[serde(default = "default_knock_delay_ms")]
    pub delay_ms: u64,
}

fn default_knock_delay_ms() -> u64 {
    120
}

pub fn format_port_knock(steps: &[PortKnockStep]) -> String {
    steps
        .iter()
        .map(|s| format!("{}:{}", s.protocol, s.port))
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn parse_port_knock(raw: &str) -> Vec<PortKnockStep> {
    raw.split([',', ';', ' '])
        .filter_map(|tok| {
            let t = tok.trim();
            if t.is_empty() {
                return None;
            }
            if let Some((proto, port_str)) = t.split_once(':') {
                let port: u16 = port_str.trim().parse().ok()?;
                if port == 0 {
                    return None;
                }
                let protocol = if proto.trim().eq_ignore_ascii_case("udp") {
                    KnockProtocol::Udp
                } else {
                    KnockProtocol::Tcp
                };
                Some(PortKnockStep {
                    protocol,
                    port,
                    delay_ms: default_knock_delay_ms(),
                })
            } else {
                let port: u16 = t.parse().ok()?;
                (port > 0).then_some(PortKnockStep {
                    protocol: KnockProtocol::Tcp,
                    port,
                    delay_ms: default_knock_delay_ms(),
                })
            }
        })
        .collect()
}

pub fn format_env_vars(vars: &[(String, String)]) -> String {
    vars.iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn parse_env_vars(raw: &str) -> Vec<(String, String)> {
    raw.split([',', ';', '\n'])
        .filter_map(|part| {
            let trimmed = part.trim();
            let (k, v) = trimmed.split_once('=')?;
            let k = k.trim();
            if k.is_empty() {
                return None;
            }
            Some((k.to_string(), v.trim().to_string()))
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedTunnelRule {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub direction: TunnelDirection,
    #[serde(default)]
    pub local_port: u16,
    #[serde(default)]
    pub remote_host: String,
    #[serde(default)]
    pub remote_port: u16,
    #[serde(default = "default_tcp")]
    pub protocol: String,
    #[serde(default)]
    pub auto_start: bool,
}

fn default_tcp() -> String {
    "tcp".to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    #[default]
    Password,
    VaultKey,
    KeyFile,
    Agent,
}

impl std::fmt::Display for AuthMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Password => write!(f, "Password (or Auto-Detect)"),
            Self::VaultKey => write!(f, "Vault SSH Key"),
            Self::KeyFile => write!(f, "Private Key File"),
            Self::Agent => write!(f, "SSH Agent"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostGroup {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub identity_id: Option<String>,
    #[serde(default)]
    pub proxy_jump_id: Option<String>,
    #[serde(default)]
    pub default_port: Option<u16>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityEntry {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub auth_method: AuthMethod,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(default)]
    pub ssh_key_id: Option<String>,
    #[serde(default)]
    pub private_key_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passphrase: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IdentitySummary {
    pub id: String,
    pub name: String,
    pub username: String,
    pub auth_method: AuthMethod,
    pub ssh_key_id: Option<String>,
    pub private_key_path: String,
    pub has_password: bool,
    pub has_passphrase: bool,
}

impl From<&IdentityEntry> for IdentitySummary {
    fn from(e: &IdentityEntry) -> Self {
        Self {
            id: e.id.clone(),
            name: e.name.clone(),
            username: e.username.clone(),
            auth_method: e.auth_method,
            ssh_key_id: e.ssh_key_id.clone(),
            private_key_path: e.private_key_path.clone(),
            has_password: e.password.as_ref().is_some_and(|p| !p.is_empty()),
            has_passphrase: e.passphrase.as_ref().is_some_and(|p| !p.is_empty()),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct IdentityInput {
    pub name: String,
    pub username: String,
    pub auth_method: AuthMethod,
    pub password: Option<String>,
    pub ssh_key_id: Option<String>,
    pub private_key_path: String,
    pub passphrase: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnippetPackage {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub snippets: Vec<CommandSnippet>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HighlightRule {
    pub id: String,
    pub name: String,
    pub pattern: String,
    pub color_hex: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppLockConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password_hash: Option<String>,
    #[serde(default)]
    pub idle_timeout_mins: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SyncBackend {
    #[default]
    None,
    LocalFolder,
    WebDav,
}

impl std::fmt::Display for SyncBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "Disabled"),
            Self::LocalFolder => write!(f, "Local Sync Folder (E2EE)"),
            Self::WebDav => write!(f, "WebDAV Server (E2EE)"),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncConfig {
    #[serde(default)]
    pub backend: SyncBackend,
    #[serde(default)]
    pub folder_path: String,
    #[serde(default)]
    pub webdav_url: String,
    #[serde(default)]
    pub webdav_username: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypted_webdav_password: Option<String>,
    #[serde(default)]
    pub last_sync_at: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SyncMergeStats {
    pub servers_added: usize,
    pub groups_added: usize,
    pub identities_added: usize,
    pub packages_added: usize,
    pub keys_added: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerEntry {
    pub id: String,
    pub name: String,
    pub host: String,
    pub username: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub os: Option<String>,
    #[serde(default)]
    pub zerotier: String,
    #[serde(default, alias = "private_key_path", alias = "privateKey")]
    pub private_key_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,

    // v3 extended fields (default cleanly when importing v2/mobile .glyph backups)
    #[serde(default)]
    pub auth_method: AuthMethod,
    #[serde(default)]
    pub ssh_key_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passphrase: Option<String>,
    #[serde(default)]
    pub proxy_jump_id: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,

    // v3 Termius-parity extended fields
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub identity_id: Option<String>,
    #[serde(default)]
    pub forward_agent: bool,
    #[serde(default = "default_keepalive")]
    pub keepalive_secs: u16,
    #[serde(default)]
    pub startup_command: String,
    #[serde(default)]
    pub env_vars: Vec<(String, String)>,
    #[serde(default)]
    pub port_knock: Vec<PortKnockStep>,
    #[serde(default)]
    pub saved_tunnels: Vec<SavedTunnelRule>,
    #[serde(default)]
    pub auto_log: bool,
}

fn default_port() -> u16 {
    22
}

fn default_keepalive() -> u16 {
    15
}

/// Safe public view of a server without decrypted secret fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerSummary {
    pub id: String,
    pub name: String,
    pub host: String,
    pub username: String,
    pub port: u16,
    pub os: Option<String>,
    pub zerotier: String,
    pub private_key_path: String,
    pub auth_method: AuthMethod,
    pub ssh_key_id: Option<String>,
    pub proxy_jump_id: Option<String>,
    pub tags: Vec<String>,
    pub has_password: bool,
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub identity_id: Option<String>,
    #[serde(default)]
    pub forward_agent: bool,
    #[serde(default = "default_keepalive")]
    pub keepalive_secs: u16,
    #[serde(default)]
    pub startup_command: String,
    #[serde(default)]
    pub env_vars: Vec<(String, String)>,
    #[serde(default)]
    pub port_knock: Vec<PortKnockStep>,
    #[serde(default)]
    pub saved_tunnels: Vec<SavedTunnelRule>,
    #[serde(default)]
    pub auto_log: bool,
}

impl From<&ServerEntry> for ServerSummary {
    fn from(s: &ServerEntry) -> Self {
        Self {
            id: s.id.clone(),
            name: s.name.clone(),
            host: s.host.clone(),
            username: s.username.clone(),
            port: s.port,
            os: s.os.clone(),
            zerotier: s.zerotier.clone(),
            private_key_path: s.private_key_path.clone(),
            auth_method: s.auth_method,
            ssh_key_id: s.ssh_key_id.clone(),
            proxy_jump_id: s.proxy_jump_id.clone(),
            tags: s.tags.clone(),
            has_password: s.password.as_ref().is_some_and(|p| !p.is_empty()),
            group_id: s.group_id.clone(),
            identity_id: s.identity_id.clone(),
            forward_agent: s.forward_agent,
            keepalive_secs: s.keepalive_secs,
            startup_command: s.startup_command.clone(),
            env_vars: s.env_vars.clone(),
            port_knock: s.port_knock.clone(),
            saved_tunnels: s.saved_tunnels.clone(),
            auto_log: s.auto_log,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServerConfigInput {
    pub id: Option<String>,
    pub name: String,
    pub host: String,
    pub username: String,
    pub port: u16,
    pub password: Option<String>,
    pub zerotier: String,
    pub private_key_path: String,
    pub auth_method: AuthMethod,
    pub ssh_key_id: Option<String>,
    pub passphrase: Option<String>,
    pub proxy_jump_id: Option<String>,
    pub tags: Vec<String>,
    pub group_id: Option<String>,
    pub identity_id: Option<String>,
    pub forward_agent: bool,
    pub keepalive_secs: u16,
    pub startup_command: String,
    pub env_vars: Vec<(String, String)>,
    pub port_knock: Vec<PortKnockStep>,
    pub saved_tunnels: Vec<SavedTunnelRule>,
    pub auto_log: bool,
}

impl Default for ServerConfigInput {
    fn default() -> Self {
        Self {
            id: None,
            name: String::new(),
            host: String::new(),
            username: String::new(),
            port: 22,
            password: None,
            zerotier: String::new(),
            private_key_path: String::new(),
            auth_method: AuthMethod::Password,
            ssh_key_id: None,
            passphrase: None,
            proxy_jump_id: None,
            tags: Vec::new(),
            group_id: None,
            identity_id: None,
            forward_agent: false,
            keepalive_secs: default_keepalive(),
            startup_command: String::new(),
            env_vars: Vec::new(),
            port_knock: Vec::new(),
            saved_tunnels: Vec::new(),
            auto_log: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub private_key_path: Option<String>,
    pub private_key_pem: Option<String>,
    pub passphrase: Option<String>,
    pub auth_method: AuthMethod,
    pub zerotier: String,
    pub proxy_jump: Option<Box<ConnectionConfig>>,
    #[serde(default)]
    pub forward_agent: bool,
    #[serde(default = "default_keepalive")]
    pub keepalive_secs: u16,
    #[serde(default)]
    pub env_vars: Vec<(String, String)>,
    #[serde(default)]
    pub port_knock: Vec<PortKnockStep>,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: 22,
            username: "root".to_string(),
            password: None,
            private_key_path: None,
            private_key_pem: None,
            passphrase: None,
            auth_method: AuthMethod::Password,
            zerotier: String::new(),
            proxy_jump: None,
            forward_agent: false,
            keepalive_secs: default_keepalive(),
            env_vars: Vec::new(),
            port_knock: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SshKeyAlgorithm {
    #[default]
    Ed25519,
    Rsa4096,
    EcdsaP256,
}

impl std::fmt::Display for SshKeyAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ed25519 => write!(f, "Ed25519"),
            Self::Rsa4096 => write!(f, "RSA-4096"),
            Self::EcdsaP256 => write!(f, "ECDSA-P256"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshKeyEntry {
    pub id: String,
    pub name: String,
    pub algorithm: String,
    pub fingerprint_sha256: String,
    pub public_key_openssh: String,
    /// Encrypted private key payload stored in vault.
    pub encrypted_private_key: String,
    pub has_passphrase: bool,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct SshKeySummary {
    pub id: String,
    pub name: String,
    pub algorithm: String,
    pub fingerprint_sha256: String,
    pub public_key_openssh: String,
    pub has_passphrase: bool,
    pub created_at: String,
}

impl From<&SshKeyEntry> for SshKeySummary {
    fn from(k: &SshKeyEntry) -> Self {
        Self {
            id: k.id.clone(),
            name: k.name.clone(),
            algorithm: k.algorithm.clone(),
            fingerprint_sha256: k.fingerprint_sha256.clone(),
            public_key_openssh: k.public_key_openssh.clone(),
            has_passphrase: k.has_passphrase,
            created_at: k.created_at.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownHostEntry {
    pub host_port: String,
    pub algorithm: String,
    pub fingerprint_sha256: String,
    pub added_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostKeyStatus {
    Trusted,
    Unknown,
    Mismatch {
        expected: String,
        actual: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretEntry {
    pub id: String,
    pub name: String,
    /// Encrypted in local vault; plain when exported inside an encrypted `.glyph` file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SecretSummary {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandSnippet {
    pub id: u64,
    pub name: String,
    pub cmd: String,
    #[serde(default)]
    pub uses: u64,
}

#[derive(Debug, Clone, Default)]
pub struct CpuCoreStat {
    pub id: String,
    pub pct: f32,
}

#[derive(Debug, Clone, Default)]
pub struct TempReading {
    pub label: String,
    pub celsius: f32,
}

#[derive(Debug, Clone, Default)]
pub struct MemStats {
    pub total_mb: f32,
    pub used_mb: f32,
    pub free_mb: f32,
    pub buff_cache_mb: f32,
    pub swap_total_mb: f32,
    pub swap_used_mb: f32,
    pub swap_free_mb: f32,
}

#[derive(Debug, Clone, Default)]
pub struct DiskMount {
    pub source: String,
    pub size: String,
    pub used: String,
    pub avail: String,
    pub pct: f32,
    pub mountpoint: String,
}

#[derive(Debug, Clone, Default)]
pub struct NetInterfaceStat {
    pub iface: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_bps: f64,
    pub tx_bps: f64,
}

#[derive(Debug, Clone, Default)]
pub struct GpuInfo {
    pub index: String,
    pub name: String,
    pub util_pct: f32,
    pub temp_c: f32,
    pub mem_total_mb: f32,
    pub mem_used_mb: f32,
}

#[derive(Debug, Clone)]
pub struct TelemetrySnapshot {
    pub cpu_pct: f32,
    pub mem_pct: f32,
    pub disk_pct: f32,
    /// None = not reported yet, Some(-1.0) = NO_GPU, Some(val) = average GPU utilization %
    pub gpu_pct: Option<f32>,
    pub uptime: String,
    pub active_users: String,
    pub load_avg: Option<[String; 3]>,
    pub tasks_line: String,
    pub cores: Vec<CpuCoreStat>,
    pub temps: Vec<TempReading>,
    pub mem: MemStats,
    pub disks: Vec<DiskMount>,
    pub net_interfaces: Vec<NetInterfaceStat>,
    pub total_rx_bps: f64,
    pub total_tx_bps: f64,
    pub gpus: Vec<GpuInfo>,
    pub no_gpu_detected: bool,
}

impl Default for TelemetrySnapshot {
    fn default() -> Self {
        Self {
            cpu_pct: 0.0,
            mem_pct: 0.0,
            disk_pct: 0.0,
            gpu_pct: None,
            uptime: "Loading...".to_string(),
            active_users: "0".to_string(),
            load_avg: None,
            tasks_line: String::new(),
            cores: Vec::new(),
            temps: Vec::new(),
            mem: MemStats::default(),
            disks: Vec::new(),
            net_interfaces: Vec::new(),
            total_rx_bps: 0.0,
            total_tx_bps: 0.0,
            gpus: Vec::new(),
            no_gpu_detected: false,
        }
    }
}
