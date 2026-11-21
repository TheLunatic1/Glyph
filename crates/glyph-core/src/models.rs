use serde::{Deserialize, Serialize};

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
}

fn default_port() -> u16 {
    22
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
        }
    }
}

#[derive(Debug, Clone, Default)]
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
