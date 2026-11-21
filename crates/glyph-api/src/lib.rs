use anyhow::{anyhow, Context, Result};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use directories::BaseDirs;
use glyph_core::containers::{ContainerAction, ContainerDetails};
use glyph_core::crypto::write_restricted_file;
use glyph_core::models::{
    CommandSnippet, ConnectionConfig, IdentitySummary, KnownHostEntry, SnippetPackage,
    SshKeySummary, TelemetrySnapshot,
};
use glyph_core::sftp::{sftp_transfer_between_sessions, SftpEntry};
use glyph_core::ssh::{ConnectOutcome, HostKeyPrompt, SessionEvent, SshSession};
use glyph_core::telemetry::{TelemetryTracker, GLYPH_STAT_CMD};
use glyph_core::tunnels::TunnelInfo;
use glyph_core::vault::VaultManager;
use rand::rngs::SysRng;
use rand::TryRng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::task::JoinHandle;

static XFER_SEQ: AtomicU64 = AtomicU64::new(1);

pub const DEFAULT_API_PORT: u16 = 15354;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiHostKeyPolicy {
    AutoTrustFirstUse,
    StrictKnownHosts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiUiTab {
    Dashboard,
    Terminal,
    Sftp,
    Commands,
    Secrets,
    Tunnels,
    Containers,
    Vnc,
}

#[derive(Clone)]
pub enum ApiUiEvent {
    SessionConnected {
        server_id: String,
        server_name: String,
        os: Option<String>,
        session: SshSession,
    },
    SessionDisconnected {
        server_id: String,
    },
    SwitchTab {
        server_id: String,
        tab: ApiUiTab,
    },
    SecretsChanged {
        server_id: String,
    },
    CommandsChanged {
        server_id: String,
    },
    TunnelsChanged {
        server_id: String,
    },
    VaultChanged,
    AgentCommandStarted {
        server_id: String,
        command: String,
    },
    AgentCommandOutput {
        server_id: String,
        output: String,
    },
    AgentTelemetryUpdated {
        server_id: String,
        snapshot: Box<TelemetrySnapshot>,
    },
    AgentSftpFile {
        server_id: String,
        action: String,
        path: String,
        content: String,
    },
    AgentSftpDir {
        server_id: String,
        path: String,
        entries: Vec<SftpEntry>,
        status: String,
    },
    AgentSftpRefresh {
        server_id: String,
        dir: String,
        status: String,
    },
    AgentTunnelDirection {
        server_id: String,
        direction: String,
    },
    AgentContainerDetail {
        server_id: String,
        container_id: String,
        show_inspect: bool,
        logs: Option<String>,
        inspect: Option<Box<ContainerDetails>>,
    },
    AgentComposeResult {
        server_id: String,
        dir: String,
        output: String,
    },
    AgentOpenVnc {
        server_id: String,
        host: String,
        port: u16,
        auto_connect: bool,
    },
}

impl std::fmt::Debug for ApiUiEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SessionConnected {
                server_id,
                server_name,
                os,
                ..
            } => f
                .debug_struct("SessionConnected")
                .field("server_id", server_id)
                .field("server_name", server_name)
                .field("os", os)
                .finish(),
            Self::SessionDisconnected { server_id } => f
                .debug_struct("SessionDisconnected")
                .field("server_id", server_id)
                .finish(),
            Self::SwitchTab { server_id, tab } => f
                .debug_struct("SwitchTab")
                .field("server_id", server_id)
                .field("tab", tab)
                .finish(),
            Self::SecretsChanged { server_id } => f
                .debug_struct("SecretsChanged")
                .field("server_id", server_id)
                .finish(),
            Self::CommandsChanged { server_id } => f
                .debug_struct("CommandsChanged")
                .field("server_id", server_id)
                .finish(),
            Self::TunnelsChanged { server_id } => f
                .debug_struct("TunnelsChanged")
                .field("server_id", server_id)
                .finish(),
            Self::VaultChanged => f.write_str("VaultChanged"),
            Self::AgentCommandStarted { server_id, command } => f
                .debug_struct("AgentCommandStarted")
                .field("server_id", server_id)
                .field("command", command)
                .finish(),
            Self::AgentCommandOutput { server_id, output } => f
                .debug_struct("AgentCommandOutput")
                .field("server_id", server_id)
                .field("output_len", &output.len())
                .finish(),
            Self::AgentTelemetryUpdated { server_id, snapshot } => f
                .debug_struct("AgentTelemetryUpdated")
                .field("server_id", server_id)
                .field("cpu_pct", &snapshot.cpu_pct)
                .finish(),
            Self::AgentSftpFile {
                server_id,
                action,
                path,
                ..
            } => f
                .debug_struct("AgentSftpFile")
                .field("server_id", server_id)
                .field("action", action)
                .field("path", path)
                .finish(),
            Self::AgentSftpDir {
                server_id,
                path,
                entries,
                ..
            } => f
                .debug_struct("AgentSftpDir")
                .field("server_id", server_id)
                .field("path", path)
                .field("entries_len", &entries.len())
                .finish(),
            Self::AgentSftpRefresh {
                server_id,
                dir,
                status,
            } => f
                .debug_struct("AgentSftpRefresh")
                .field("server_id", server_id)
                .field("dir", dir)
                .field("status", status)
                .finish(),
            Self::AgentTunnelDirection {
                server_id,
                direction,
            } => f
                .debug_struct("AgentTunnelDirection")
                .field("server_id", server_id)
                .field("direction", direction)
                .finish(),
            Self::AgentContainerDetail {
                server_id,
                container_id,
                show_inspect,
                ..
            } => f
                .debug_struct("AgentContainerDetail")
                .field("server_id", server_id)
                .field("container_id", container_id)
                .field("show_inspect", show_inspect)
                .finish(),
            Self::AgentComposeResult {
                server_id,
                dir,
                output,
            } => f
                .debug_struct("AgentComposeResult")
                .field("server_id", server_id)
                .field("dir", dir)
                .field("output_len", &output.len())
                .finish(),
            Self::AgentOpenVnc {
                server_id,
                host,
                port,
                auto_connect,
            } => f
                .debug_struct("AgentOpenVnc")
                .field("server_id", server_id)
                .field("host", host)
                .field("port", port)
                .field("auto_connect", auto_connect)
                .finish(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApiServerSummary {
    pub id: String,
    pub name: String,
    pub host: String,
    pub username: String,
    pub port: u16,
    pub os: Option<String>,
    pub group_id: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApiSecretSummary {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApiGroupSummary {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub tags: Vec<String>,
    pub server_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GroupCommandResult {
    pub server_id: String,
    pub server_name: String,
    pub ok: bool,
    pub output: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SnippetInput {
    pub name: String,
    pub cmd: String,
}

#[derive(Debug, Clone)]
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
}

impl ApiError {
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: msg.into(),
        }
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: msg.into(),
        }
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: msg.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}

pub type ApiResult<T> = std::result::Result<T, ApiError>;

pub trait ApiController: Send + Sync {
    fn list_servers(&self) -> BoxFuture<'_, ApiResult<Vec<ApiServerSummary>>>;
    fn connect(&self, server_id: String) -> BoxFuture<'_, ApiResult<()>>;
    fn disconnect(&self, server_id: String) -> BoxFuture<'_, ApiResult<()>>;
    fn execute(&self, server_id: String, command: String) -> BoxFuture<'_, ApiResult<String>>;
    fn telemetry(&self, server_id: String) -> BoxFuture<'_, ApiResult<TelemetrySnapshot>>;
    fn sftp_read(&self, server_id: String, path: String) -> BoxFuture<'_, ApiResult<String>>;
    fn sftp_write(
        &self,
        server_id: String,
        path: String,
        content: String,
    ) -> BoxFuture<'_, ApiResult<()>>;
    fn sftp_list(&self, server_id: String, path: String) -> BoxFuture<'_, ApiResult<Vec<SftpEntry>>>;
    fn sftp_mkdir(&self, server_id: String, path: String) -> BoxFuture<'_, ApiResult<()>>;
    fn sftp_rename(
        &self,
        server_id: String,
        old_path: String,
        new_path: String,
    ) -> BoxFuture<'_, ApiResult<()>>;
    fn sftp_chmod(&self, server_id: String, path: String, mode: u32) -> BoxFuture<'_, ApiResult<()>>;
    fn sftp_delete(
        &self,
        server_id: String,
        path: String,
        is_dir: bool,
    ) -> BoxFuture<'_, ApiResult<()>>;
    fn sftp_transfer(
        &self,
        source_server_id: String,
        source_path: String,
        dest_server_id: String,
        dest_path: String,
    ) -> BoxFuture<'_, ApiResult<u64>>;
    fn tunnels_list(&self, server_id: String) -> BoxFuture<'_, ApiResult<Vec<TunnelInfo>>>;
    fn tunnels_start(
        &self,
        server_id: String,
        local_port: u16,
        remote_host: String,
        remote_port: u16,
        protocol: String,
    ) -> BoxFuture<'_, ApiResult<TunnelInfo>>;
    fn tunnels_start_remote(
        &self,
        server_id: String,
        remote_bind_addr: String,
        remote_port: u16,
        local_host: String,
        local_port: u16,
    ) -> BoxFuture<'_, ApiResult<TunnelInfo>>;
    fn tunnels_start_socks(&self, server_id: String, local_port: u16) -> BoxFuture<'_, ApiResult<TunnelInfo>>;
    fn tunnels_stop(
        &self,
        server_id: String,
        local_port: u16,
        tunnel_id: Option<String>,
    ) -> BoxFuture<'_, ApiResult<()>>;
    fn secrets_list(&self, server_id: String) -> BoxFuture<'_, ApiResult<Vec<ApiSecretSummary>>>;
    fn secrets_add(
        &self,
        server_id: String,
        name: String,
        value: String,
    ) -> BoxFuture<'_, ApiResult<String>>;
    fn containers_list(&self, server_id: String) -> BoxFuture<'_, ApiResult<String>>;
    fn containers_logs(
        &self,
        server_id: String,
        container_id: String,
        tail: u32,
    ) -> BoxFuture<'_, ApiResult<String>>;
    fn containers_inspect(
        &self,
        server_id: String,
        container_id: String,
        is_running: bool,
    ) -> BoxFuture<'_, ApiResult<ContainerDetails>>;
    fn containers_action(
        &self,
        server_id: String,
        container_id: String,
        action: String,
    ) -> BoxFuture<'_, ApiResult<()>>;
    fn containers_compose(
        &self,
        server_id: String,
        dir: String,
        action: String,
    ) -> BoxFuture<'_, ApiResult<String>>;
    fn commands_list(&self, server_id: String) -> BoxFuture<'_, ApiResult<Vec<CommandSnippet>>>;
    fn commands_add(
        &self,
        server_id: String,
        name: String,
        cmd: String,
    ) -> BoxFuture<'_, ApiResult<u64>>;
    fn commands_remove(&self, server_id: String, id: u64) -> BoxFuture<'_, ApiResult<()>>;
    fn groups_list(&self) -> BoxFuture<'_, ApiResult<Vec<ApiGroupSummary>>>;
    fn groups_execute(
        &self,
        group_id: String,
        command: String,
    ) -> BoxFuture<'_, ApiResult<Vec<GroupCommandResult>>>;
    fn snippet_packages_list(&self) -> BoxFuture<'_, ApiResult<Vec<SnippetPackage>>>;
    fn snippet_packages_add(
        &self,
        name: String,
        description: String,
        snippets: Vec<SnippetInput>,
    ) -> BoxFuture<'_, ApiResult<SnippetPackage>>;
    fn ssh_keys_list(&self) -> BoxFuture<'_, ApiResult<Vec<SshKeySummary>>>;
    fn identities_list(&self) -> BoxFuture<'_, ApiResult<Vec<IdentitySummary>>>;
    fn known_hosts_list(&self) -> BoxFuture<'_, ApiResult<Vec<KnownHostEntry>>>;
    fn vnc_open(
        &self,
        server_id: String,
        host: String,
        port: u16,
        auto_connect: bool,
    ) -> BoxFuture<'_, ApiResult<()>>;
}

pub fn default_mcp_token_path() -> Result<PathBuf> {
    if let Some(base) = BaseDirs::new() {
        return Ok(base.home_dir().join(".glyph").join("mcp_token"));
    }
    if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        return Ok(PathBuf::from(home).join(".glyph").join("mcp_token"));
    }
    Err(anyhow!("Unable to resolve user home directory for ~/.glyph/mcp_token"))
}

pub fn ensure_mcp_token(path: &Path) -> Result<String> {
    if path.exists() {
        if let Ok(existing) = std::fs::read_to_string(path) {
            let trimmed = existing.trim().to_string();
            if trimmed.len() >= 32 {
                return Ok(trimmed);
            }
        }
    }

    let mut bytes = [0u8; 32];
    SysRng
        .try_fill_bytes(&mut bytes)
        .map_err(|e| anyhow!("OS RNG failure while creating MCP token: {e}"))?;
    let mut token = String::with_capacity(64);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(&mut token, "{b:02x}");
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    write_restricted_file(path, token.as_bytes())?;
    Ok(token)
}

#[derive(Clone)]
struct AppState {
    token: Arc<String>,
    controller: Arc<dyn ApiController>,
}

pub struct LocalApiHandle {
    addr: SocketAddr,
    shutdown_tx: Option<oneshot::Sender<()>>,
    task: JoinHandle<()>,
}

impl LocalApiHandle {
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        let _ = self.task.await;
    }
}

pub async fn start_local_api(
    bind_addr: SocketAddr,
    token: String,
    controller: Arc<dyn ApiController>,
) -> Result<LocalApiHandle> {
    let listener = TcpListener::bind(bind_addr)
        .await
        .with_context(|| format!("Failed to bind Glyph Local API on {bind_addr}"))?;
    let local_addr = listener.local_addr()?;

    let state = AppState {
        token: Arc::new(token),
        controller,
    };

    let app = Router::new()
        .route("/api/servers", get(handle_list_servers))
        .route("/api/connect", post(handle_connect))
        .route("/api/disconnect", post(handle_disconnect))
        .route("/api/execute", post(handle_execute))
        .route("/api/telemetry", post(handle_telemetry))
        .route("/api/sftp/read", post(handle_sftp_read))
        .route("/api/sftp/write", post(handle_sftp_write))
        .route("/api/sftp/list", post(handle_sftp_list))
        .route("/api/sftp/mkdir", post(handle_sftp_mkdir))
        .route("/api/sftp/rename", post(handle_sftp_rename))
        .route("/api/sftp/chmod", post(handle_sftp_chmod))
        .route("/api/sftp/delete", post(handle_sftp_delete))
        .route("/api/sftp/transfer", post(handle_sftp_transfer))
        .route("/api/tunnels/list", post(handle_tunnels_list))
        .route("/api/tunnels/start", post(handle_tunnels_start))
        .route("/api/tunnels/start_remote", post(handle_tunnels_start_remote))
        .route("/api/tunnels/start_socks", post(handle_tunnels_start_socks))
        .route("/api/tunnels/stop", post(handle_tunnels_stop))
        .route("/api/secrets/list", post(handle_secrets_list))
        .route("/api/secrets/add", post(handle_secrets_add))
        .route("/api/containers/list", post(handle_containers_list))
        .route("/api/containers/logs", post(handle_containers_logs))
        .route("/api/containers/inspect", post(handle_containers_inspect))
        .route("/api/containers/action", post(handle_containers_action))
        .route("/api/containers/compose", post(handle_containers_compose))
        .route("/api/commands/list", post(handle_commands_list))
        .route("/api/commands/add", post(handle_commands_add))
        .route("/api/commands/remove", post(handle_commands_remove))
        .route("/api/groups", get(handle_groups_list))
        .route("/api/groups/execute", post(handle_groups_execute))
        .route("/api/snippet_packages", get(handle_snippet_packages_list))
        .route("/api/snippet_packages/add", post(handle_snippet_packages_add))
        .route("/api/ssh_keys", get(handle_ssh_keys_list))
        .route("/api/identities", get(handle_identities_list))
        .route("/api/known_hosts", get(handle_known_hosts_list))
        .route("/api/vnc/open", post(handle_vnc_open))
        .layer(middleware::from_fn_with_state(state.clone(), require_bearer))
        .with_state(state);

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let task = tokio::spawn(async move {
        let server = axum::serve(listener, app).with_graceful_shutdown(async {
            let _ = shutdown_rx.await;
        });
        if let Err(err) = server.await {
            log::error!("Glyph Local API server stopped with error: {err}");
        }
    });

    Ok(LocalApiHandle {
        addr: local_addr,
        shutdown_tx: Some(shutdown_tx),
        task,
    })
}

async fn require_bearer(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    let provided = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|raw| raw.strip_prefix("Bearer "))
        .unwrap_or("");

    if !constant_time_eq(provided.as_bytes(), state.token.as_bytes()) {
        return (StatusCode::UNAUTHORIZED, Json(json!({ "error": "Unauthorized" })))
            .into_response();
    }

    next.run(request).await
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    let max_len = a.len().max(b.len());
    let mut diff = a.len() ^ b.len();
    for i in 0..max_len {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        diff |= usize::from(x ^ y);
    }
    diff == 0 && !b.is_empty()
}

// ── Request Payloads ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServerIdBody {
    server_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExecuteBody {
    server_id: String,
    command: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SftpReadBody {
    server_id: String,
    path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SftpWriteBody {
    server_id: String,
    path: String,
    content: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TunnelsStartBody {
    server_id: String,
    #[serde(default)]
    local_port: u16,
    #[serde(default = "default_localhost")]
    remote_host: String,
    remote_port: u16,
    #[serde(default = "default_tcp")]
    protocol: String,
}

fn default_localhost() -> String {
    "127.0.0.1".to_string()
}

fn default_tcp() -> String {
    "TCP".to_string()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TunnelsStopBody {
    server_id: String,
    #[serde(default)]
    local_port: u16,
    #[serde(default)]
    tunnel_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TunnelsStartRemoteBody {
    server_id: String,
    #[serde(default = "default_localhost")]
    remote_bind_addr: String,
    remote_port: u16,
    #[serde(default = "default_localhost")]
    local_host: String,
    local_port: u16,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TunnelsStartSocksBody {
    server_id: String,
    #[serde(default)]
    local_port: u16,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SftpListBody {
    server_id: String,
    #[serde(default = "default_root")]
    path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SftpPathBody {
    server_id: String,
    path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SftpRenameBody {
    server_id: String,
    old_path: String,
    new_path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SftpChmodBody {
    server_id: String,
    path: String,
    mode: Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SftpDeleteBody {
    server_id: String,
    path: String,
    #[serde(default)]
    is_dir: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SftpTransferBody {
    source_server_id: String,
    source_path: String,
    dest_server_id: String,
    dest_path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContainersLogsBody {
    server_id: String,
    container_id: String,
    #[serde(default = "default_tail")]
    tail: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContainersInspectBody {
    server_id: String,
    container_id: String,
    #[serde(default = "default_true")]
    is_running: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContainersActionBody {
    server_id: String,
    container_id: String,
    action: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContainersComposeBody {
    server_id: String,
    dir: String,
    action: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GroupsExecuteBody {
    group_id: String,
    command: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnippetPackagesAddBody {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    snippets: Vec<SnippetInput>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VncOpenBody {
    server_id: String,
    #[serde(default = "default_localhost")]
    host: String,
    #[serde(default = "default_vnc_port")]
    port: u16,
    #[serde(default = "default_true")]
    auto_connect: bool,
}

fn default_root() -> String {
    "/".to_string()
}

fn default_tail() -> u32 {
    200
}

fn default_vnc_port() -> u16 {
    5900
}

fn default_true() -> bool {
    true
}

fn parse_chmod_mode(raw: &Value) -> ApiResult<u32> {
    let mode = match raw {
        Value::String(s) => {
            let cleaned = s.trim().trim_start_matches("0o").trim_start_matches("0O");
            if cleaned.is_empty() {
                return Err(ApiError::bad_request("mode cannot be empty"));
            }
            u32::from_str_radix(cleaned, 8)
                .map_err(|_| ApiError::bad_request(format!("Invalid octal mode \"{s}\"")))?
        }
        Value::Number(n) => {
            let v = n
                .as_u64()
                .ok_or_else(|| ApiError::bad_request("mode must be an octal string or integer"))?;
            u32::try_from(v).map_err(|_| ApiError::bad_request("mode out of range"))?
        }
        _ => {
            return Err(ApiError::bad_request(
                "mode must be an octal string (e.g. \"755\") or integer",
            ))
        }
    };
    if mode > 0o7777 {
        return Err(ApiError::bad_request("mode must be <= 07777"));
    }
    Ok(mode)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SecretsAddBody {
    server_id: String,
    name: String,
    value: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommandsAddBody {
    server_id: String,
    name: String,
    cmd: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommandsRemoveBody {
    server_id: String,
    id: u64,
}

// ── Route Handlers ────────────────────────────────────────────────────────────

async fn handle_list_servers(State(state): State<AppState>) -> ApiResult<Json<Vec<ApiServerSummary>>> {
    let list = state.controller.list_servers().await?;
    Ok(Json(list))
}

async fn handle_connect(
    State(state): State<AppState>,
    Json(body): Json<ServerIdBody>,
) -> ApiResult<Json<Value>> {
    state.controller.connect(body.server_id).await?;
    Ok(Json(json!({ "success": true })))
}

async fn handle_execute(
    State(state): State<AppState>,
    Json(body): Json<ExecuteBody>,
) -> ApiResult<Json<Value>> {
    let output = state
        .controller
        .execute(body.server_id, body.command)
        .await?;
    Ok(Json(json!({ "output": output })))
}

async fn handle_sftp_read(
    State(state): State<AppState>,
    Json(body): Json<SftpReadBody>,
) -> ApiResult<Json<Value>> {
    let content = state.controller.sftp_read(body.server_id, body.path).await?;
    Ok(Json(json!({ "content": content })))
}

async fn handle_sftp_write(
    State(state): State<AppState>,
    Json(body): Json<SftpWriteBody>,
) -> ApiResult<Json<Value>> {
    state
        .controller
        .sftp_write(body.server_id, body.path, body.content)
        .await?;
    Ok(Json(json!({ "success": true })))
}

async fn handle_tunnels_start(
    State(state): State<AppState>,
    Json(body): Json<TunnelsStartBody>,
) -> ApiResult<Json<TunnelInfo>> {
    let info = state
        .controller
        .tunnels_start(
            body.server_id,
            body.local_port,
            body.remote_host,
            body.remote_port,
            body.protocol,
        )
        .await?;
    Ok(Json(info))
}

async fn handle_tunnels_stop(
    State(state): State<AppState>,
    Json(body): Json<TunnelsStopBody>,
) -> ApiResult<Json<Value>> {
    state
        .controller
        .tunnels_stop(body.server_id, body.local_port, body.tunnel_id)
        .await?;
    Ok(Json(json!({ "success": true })))
}

async fn handle_disconnect(
    State(state): State<AppState>,
    Json(body): Json<ServerIdBody>,
) -> ApiResult<Json<Value>> {
    state.controller.disconnect(body.server_id).await?;
    Ok(Json(json!({ "success": true })))
}

async fn handle_telemetry(
    State(state): State<AppState>,
    Json(body): Json<ServerIdBody>,
) -> ApiResult<Json<TelemetrySnapshot>> {
    let snap = state.controller.telemetry(body.server_id).await?;
    Ok(Json(snap))
}

async fn handle_sftp_list(
    State(state): State<AppState>,
    Json(body): Json<SftpListBody>,
) -> ApiResult<Json<Vec<SftpEntry>>> {
    let entries = state.controller.sftp_list(body.server_id, body.path).await?;
    Ok(Json(entries))
}

async fn handle_sftp_mkdir(
    State(state): State<AppState>,
    Json(body): Json<SftpPathBody>,
) -> ApiResult<Json<Value>> {
    state.controller.sftp_mkdir(body.server_id, body.path).await?;
    Ok(Json(json!({ "success": true })))
}

async fn handle_sftp_rename(
    State(state): State<AppState>,
    Json(body): Json<SftpRenameBody>,
) -> ApiResult<Json<Value>> {
    state
        .controller
        .sftp_rename(body.server_id, body.old_path, body.new_path)
        .await?;
    Ok(Json(json!({ "success": true })))
}

async fn handle_sftp_chmod(
    State(state): State<AppState>,
    Json(body): Json<SftpChmodBody>,
) -> ApiResult<Json<Value>> {
    let mode = parse_chmod_mode(&body.mode)?;
    state
        .controller
        .sftp_chmod(body.server_id, body.path, mode)
        .await?;
    Ok(Json(json!({ "success": true, "mode": format!("{mode:04o}") })))
}

async fn handle_sftp_delete(
    State(state): State<AppState>,
    Json(body): Json<SftpDeleteBody>,
) -> ApiResult<Json<Value>> {
    state
        .controller
        .sftp_delete(body.server_id, body.path, body.is_dir)
        .await?;
    Ok(Json(json!({ "success": true })))
}

async fn handle_sftp_transfer(
    State(state): State<AppState>,
    Json(body): Json<SftpTransferBody>,
) -> ApiResult<Json<Value>> {
    let bytes = state
        .controller
        .sftp_transfer(
            body.source_server_id,
            body.source_path,
            body.dest_server_id,
            body.dest_path,
        )
        .await?;
    Ok(Json(json!({ "success": true, "bytesTransferred": bytes })))
}

async fn handle_tunnels_list(
    State(state): State<AppState>,
    Json(body): Json<ServerIdBody>,
) -> ApiResult<Json<Vec<TunnelInfo>>> {
    let list = state.controller.tunnels_list(body.server_id).await?;
    Ok(Json(list))
}

async fn handle_tunnels_start_remote(
    State(state): State<AppState>,
    Json(body): Json<TunnelsStartRemoteBody>,
) -> ApiResult<Json<TunnelInfo>> {
    let info = state
        .controller
        .tunnels_start_remote(
            body.server_id,
            body.remote_bind_addr,
            body.remote_port,
            body.local_host,
            body.local_port,
        )
        .await?;
    Ok(Json(info))
}

async fn handle_tunnels_start_socks(
    State(state): State<AppState>,
    Json(body): Json<TunnelsStartSocksBody>,
) -> ApiResult<Json<TunnelInfo>> {
    let info = state
        .controller
        .tunnels_start_socks(body.server_id, body.local_port)
        .await?;
    Ok(Json(info))
}

async fn handle_containers_logs(
    State(state): State<AppState>,
    Json(body): Json<ContainersLogsBody>,
) -> ApiResult<Json<Value>> {
    let logs = state
        .controller
        .containers_logs(body.server_id, body.container_id, body.tail)
        .await?;
    Ok(Json(json!({ "logs": logs })))
}

async fn handle_containers_inspect(
    State(state): State<AppState>,
    Json(body): Json<ContainersInspectBody>,
) -> ApiResult<Json<ContainerDetails>> {
    let details = state
        .controller
        .containers_inspect(body.server_id, body.container_id, body.is_running)
        .await?;
    Ok(Json(details))
}

async fn handle_containers_action(
    State(state): State<AppState>,
    Json(body): Json<ContainersActionBody>,
) -> ApiResult<Json<Value>> {
    state
        .controller
        .containers_action(body.server_id, body.container_id, body.action)
        .await?;
    Ok(Json(json!({ "success": true })))
}

async fn handle_containers_compose(
    State(state): State<AppState>,
    Json(body): Json<ContainersComposeBody>,
) -> ApiResult<Json<Value>> {
    let output = state
        .controller
        .containers_compose(body.server_id, body.dir, body.action)
        .await?;
    Ok(Json(json!({ "output": output })))
}

async fn handle_groups_list(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<ApiGroupSummary>>> {
    let groups = state.controller.groups_list().await?;
    Ok(Json(groups))
}

async fn handle_groups_execute(
    State(state): State<AppState>,
    Json(body): Json<GroupsExecuteBody>,
) -> ApiResult<Json<Vec<GroupCommandResult>>> {
    let results = state
        .controller
        .groups_execute(body.group_id, body.command)
        .await?;
    Ok(Json(results))
}

async fn handle_snippet_packages_list(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<SnippetPackage>>> {
    let pkgs = state.controller.snippet_packages_list().await?;
    Ok(Json(pkgs))
}

async fn handle_snippet_packages_add(
    State(state): State<AppState>,
    Json(body): Json<SnippetPackagesAddBody>,
) -> ApiResult<Json<SnippetPackage>> {
    let pkg = state
        .controller
        .snippet_packages_add(body.name, body.description, body.snippets)
        .await?;
    Ok(Json(pkg))
}

async fn handle_ssh_keys_list(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<SshKeySummary>>> {
    let keys = state.controller.ssh_keys_list().await?;
    Ok(Json(keys))
}

async fn handle_identities_list(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<IdentitySummary>>> {
    let ids = state.controller.identities_list().await?;
    Ok(Json(ids))
}

async fn handle_known_hosts_list(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<KnownHostEntry>>> {
    let kh = state.controller.known_hosts_list().await?;
    Ok(Json(kh))
}

async fn handle_vnc_open(
    State(state): State<AppState>,
    Json(body): Json<VncOpenBody>,
) -> ApiResult<Json<Value>> {
    state
        .controller
        .vnc_open(
            body.server_id,
            body.host.clone(),
            body.port,
            body.auto_connect,
        )
        .await?;
    Ok(Json(json!({
        "success": true,
        "host": body.host,
        "port": body.port,
        "autoConnect": body.auto_connect,
    })))
}

async fn handle_secrets_list(
    State(state): State<AppState>,
    Json(body): Json<ServerIdBody>,
) -> ApiResult<Json<Vec<ApiSecretSummary>>> {
    let items = state.controller.secrets_list(body.server_id).await?;
    Ok(Json(items))
}

async fn handle_secrets_add(
    State(state): State<AppState>,
    Json(body): Json<SecretsAddBody>,
) -> ApiResult<Json<Value>> {
    let id = state
        .controller
        .secrets_add(body.server_id, body.name, body.value)
        .await?;
    Ok(Json(json!({ "success": true, "id": id })))
}

async fn handle_containers_list(
    State(state): State<AppState>,
    Json(body): Json<ServerIdBody>,
) -> ApiResult<Json<Value>> {
    let output = state.controller.containers_list(body.server_id).await?;
    Ok(Json(json!({ "output": output })))
}

async fn handle_commands_list(
    State(state): State<AppState>,
    Json(body): Json<ServerIdBody>,
) -> ApiResult<Json<Value>> {
    let list = state.controller.commands_list(body.server_id).await?;
    Ok(Json(json!({ "commands": list })))
}

async fn handle_commands_add(
    State(state): State<AppState>,
    Json(body): Json<CommandsAddBody>,
) -> ApiResult<Json<Value>> {
    let id = state
        .controller
        .commands_add(body.server_id, body.name, body.cmd)
        .await?;
    Ok(Json(json!({ "success": true, "id": id })))
}

async fn handle_commands_remove(
    State(state): State<AppState>,
    Json(body): Json<CommandsRemoveBody>,
) -> ApiResult<Json<Value>> {
    state
        .controller
        .commands_remove(body.server_id, body.id)
        .await?;
    Ok(Json(json!({ "success": true })))
}

// ── Shared Vault + SshSession Backend (Desktop & Headless E2E) ────────────────

pub struct LocalVaultApiController {
    vault: Arc<StdMutex<VaultManager>>,
    sessions: Arc<Mutex<HashMap<String, SshSession>>>,
    ui_tx: Option<mpsc::UnboundedSender<ApiUiEvent>>,
    host_key_policy: ApiHostKeyPolicy,
}

impl LocalVaultApiController {
    pub fn new(
        vault: Arc<StdMutex<VaultManager>>,
        sessions: Arc<Mutex<HashMap<String, SshSession>>>,
        ui_tx: Option<mpsc::UnboundedSender<ApiUiEvent>>,
        host_key_policy: ApiHostKeyPolicy,
    ) -> Self {
        Self {
            vault,
            sessions,
            ui_tx,
            host_key_policy,
        }
    }

    fn emit_ui(&self, event: ApiUiEvent) {
        if let Some(tx) = &self.ui_tx {
            let _ = tx.send(event);
        }
    }

    fn require_server_exists(&self, server_id: &str) -> ApiResult<String> {
        let id = server_id.trim();
        if id.is_empty() {
            return Err(ApiError::bad_request("serverId is required"));
        }
        let guard = self
            .vault
            .lock()
            .map_err(|_| ApiError::internal("Vault mutex poisoned"))?;
        let server = guard
            .get_server(id)
            .ok_or_else(|| ApiError::not_found(format!("Server \"{id}\" not found")))?;
        Ok(server.name)
    }

    async fn ensure_connected(&self, server_id: &str) -> ApiResult<SshSession> {
        let server_name = self.require_server_exists(server_id)?;

        if let Some(existing) = self.sessions.lock().await.get(server_id).cloned() {
            if existing.is_connected().await {
                return Ok(existing);
            }
            self.sessions.lock().await.remove(server_id);
        }

        let (config, known_hosts): (ConnectionConfig, HashMap<String, String>) = {
            let guard = self
                .vault
                .lock()
                .map_err(|_| ApiError::internal("Vault mutex poisoned"))?;
            let cfg = guard
                .get_connection_config(server_id)
                .map_err(|e| ApiError::not_found(e.to_string()))?;
            let kh = guard.known_hosts_map();
            (cfg, kh)
        };

        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<SessionEvent>();
        let (hk_tx, mut hk_rx) = mpsc::unbounded_channel::<HostKeyPrompt>();
        let policy = self.host_key_policy;

        // Drain session status events when running headlessly via API.
        tokio::spawn(async move { while event_rx.recv().await.is_some() {} });

        // Handle TOFU host key prompts safely without stalling.
        tokio::spawn(async move {
            while let Some(prompt) = hk_rx.recv().await {
                let accept = match policy {
                    ApiHostKeyPolicy::AutoTrustFirstUse => prompt.mismatch_expected.is_none(),
                    ApiHostKeyPolicy::StrictKnownHosts => false,
                };
                let _ = prompt.responder.send(accept);
            }
        });

        let outcome: ConnectOutcome = tokio::time::timeout(
            Duration::from_secs(20),
            SshSession::connect(config, known_hosts, event_tx, hk_tx),
        )
        .await
        .map_err(|_| ApiError::internal("SSH connection timed out after 20s"))?
        .map_err(|e| ApiError::internal(format!("SSH connection failed: {e}")))?;

        if !outcome.newly_trusted_hosts.is_empty() {
            if let Ok(mut guard) = self.vault.lock() {
                for rec in &outcome.newly_trusted_hosts {
                    let _ = guard.trust_host_key(
                        &rec.host,
                        rec.port,
                        &rec.algorithm,
                        &rec.fingerprint_sha256,
                    );
                }
            }
        }

        self.sessions
            .lock()
            .await
            .insert(server_id.to_string(), outcome.session.clone());
        self.emit_ui(ApiUiEvent::SessionConnected {
            server_id: server_id.to_string(),
            server_name,
            os: outcome.os,
            session: outcome.session.clone(),
        });

        Ok(outcome.session)
    }
}

fn parent_dir(path: &str) -> String {
    match path.trim_end_matches('/').rsplit_once('/') {
        Some(("", _)) | None => "/".to_string(),
        Some((dir, _)) => dir.to_string(),
    }
}

fn parse_container_action(action: &str) -> ApiResult<ContainerAction> {
    match action.trim().to_ascii_lowercase().as_str() {
        "start" => Ok(ContainerAction::Start),
        "stop" => Ok(ContainerAction::Stop),
        "restart" => Ok(ContainerAction::Restart),
        other => Err(ApiError::bad_request(format!(
            "Invalid container action \"{other}\" (expected start, stop, or restart)"
        ))),
    }
}

impl ApiController for LocalVaultApiController {
    fn list_servers(&self) -> BoxFuture<'_, ApiResult<Vec<ApiServerSummary>>> {
        Box::pin(async move {
            let guard = self
                .vault
                .lock()
                .map_err(|_| ApiError::internal("Vault mutex poisoned"))?;
            Ok(guard
                .list_servers()
                .into_iter()
                .map(|s| ApiServerSummary {
                    id: s.id,
                    name: s.name,
                    host: s.host,
                    username: s.username,
                    port: s.port,
                    os: s.os,
                    group_id: s.group_id,
                    tags: s.tags,
                })
                .collect())
        })
    }

    fn connect(&self, server_id: String) -> BoxFuture<'_, ApiResult<()>> {
        Box::pin(async move {
            let _ = self.ensure_connected(&server_id).await?;
            Ok(())
        })
    }

    fn disconnect(&self, server_id: String) -> BoxFuture<'_, ApiResult<()>> {
        Box::pin(async move {
            self.require_server_exists(&server_id)?;
            let removed = self.sessions.lock().await.remove(&server_id);
            if let Some(session) = removed {
                session.disconnect().await;
            }
            self.emit_ui(ApiUiEvent::SessionDisconnected { server_id });
            Ok(())
        })
    }

    fn execute(&self, server_id: String, command: String) -> BoxFuture<'_, ApiResult<String>> {
        Box::pin(async move {
            if command.trim().is_empty() {
                return Err(ApiError::bad_request("command is required"));
            }
            let session = self.ensure_connected(&server_id).await?;
            self.emit_ui(ApiUiEvent::SwitchTab {
                server_id: server_id.clone(),
                tab: ApiUiTab::Terminal,
            });
            self.emit_ui(ApiUiEvent::AgentCommandStarted {
                server_id: server_id.clone(),
                command: command.clone(),
            });
            match session.exec(&command).await {
                Ok(output) => {
                    self.emit_ui(ApiUiEvent::AgentCommandOutput {
                        server_id,
                        output: output.clone(),
                    });
                    Ok(output)
                }
                Err(e) => {
                    let msg = e.to_string();
                    self.emit_ui(ApiUiEvent::AgentCommandOutput {
                        server_id,
                        output: format!("\x1b[31m[Error] {msg}\x1b[0m"),
                    });
                    Err(ApiError::internal(msg))
                }
            }
        })
    }

    fn telemetry(&self, server_id: String) -> BoxFuture<'_, ApiResult<TelemetrySnapshot>> {
        Box::pin(async move {
            let session = self.ensure_connected(&server_id).await?;
            let raw = session
                .exec(GLYPH_STAT_CMD)
                .await
                .map_err(|e| ApiError::internal(e.to_string()))?;
            let snapshot = TelemetryTracker::new().parse(&raw);
            self.emit_ui(ApiUiEvent::SwitchTab {
                server_id: server_id.clone(),
                tab: ApiUiTab::Dashboard,
            });
            self.emit_ui(ApiUiEvent::AgentTelemetryUpdated {
                server_id,
                snapshot: Box::new(snapshot.clone()),
            });
            Ok(snapshot)
        })
    }

    fn sftp_read(&self, server_id: String, path: String) -> BoxFuture<'_, ApiResult<String>> {
        Box::pin(async move {
            if path.trim().is_empty() {
                return Err(ApiError::bad_request("path is required"));
            }
            let session = self.ensure_connected(&server_id).await?;
            let content = session
                .sftp_read_text(&path)
                .await
                .map_err(|e| ApiError::internal(e.to_string()))?;
            self.emit_ui(ApiUiEvent::SwitchTab {
                server_id: server_id.clone(),
                tab: ApiUiTab::Sftp,
            });
            self.emit_ui(ApiUiEvent::AgentSftpFile {
                server_id,
                action: "read".to_string(),
                path,
                content: content.clone(),
            });
            Ok(content)
        })
    }

    fn sftp_write(
        &self,
        server_id: String,
        path: String,
        content: String,
    ) -> BoxFuture<'_, ApiResult<()>> {
        Box::pin(async move {
            if path.trim().is_empty() {
                return Err(ApiError::bad_request("path is required"));
            }
            let session = self.ensure_connected(&server_id).await?;
            session
                .sftp_write_text(&path, &content)
                .await
                .map_err(|e| ApiError::internal(e.to_string()))?;
            self.emit_ui(ApiUiEvent::SwitchTab {
                server_id: server_id.clone(),
                tab: ApiUiTab::Sftp,
            });
            self.emit_ui(ApiUiEvent::AgentSftpFile {
                server_id,
                action: "write".to_string(),
                path,
                content,
            });
            Ok(())
        })
    }

    fn sftp_list(&self, server_id: String, path: String) -> BoxFuture<'_, ApiResult<Vec<SftpEntry>>> {
        Box::pin(async move {
            let target_path = if path.trim().is_empty() {
                "/".to_string()
            } else {
                path.trim().to_string()
            };
            let session = self.ensure_connected(&server_id).await?;
            let entries = session
                .sftp_readdir(&target_path)
                .await
                .map_err(|e| ApiError::internal(e.to_string()))?;
            let status = format!("[MCP Agent] Listed {target_path} ({} items)", entries.len());
            self.emit_ui(ApiUiEvent::SwitchTab {
                server_id: server_id.clone(),
                tab: ApiUiTab::Sftp,
            });
            self.emit_ui(ApiUiEvent::AgentSftpDir {
                server_id,
                path: target_path,
                entries: entries.clone(),
                status,
            });
            Ok(entries)
        })
    }

    fn sftp_mkdir(&self, server_id: String, path: String) -> BoxFuture<'_, ApiResult<()>> {
        Box::pin(async move {
            let trimmed = path.trim().to_string();
            if trimmed.is_empty() {
                return Err(ApiError::bad_request("path is required"));
            }
            let session = self.ensure_connected(&server_id).await?;
            session
                .sftp_mkdir(&trimmed)
                .await
                .map_err(|e| ApiError::internal(e.to_string()))?;
            let dir = parent_dir(&trimmed);
            self.emit_ui(ApiUiEvent::SwitchTab {
                server_id: server_id.clone(),
                tab: ApiUiTab::Sftp,
            });
            self.emit_ui(ApiUiEvent::AgentSftpRefresh {
                server_id,
                dir,
                status: format!("[MCP Agent] Created folder {trimmed}"),
            });
            Ok(())
        })
    }

    fn sftp_rename(
        &self,
        server_id: String,
        old_path: String,
        new_path: String,
    ) -> BoxFuture<'_, ApiResult<()>> {
        Box::pin(async move {
            let old_trimmed = old_path.trim().to_string();
            let new_trimmed = new_path.trim().to_string();
            if old_trimmed.is_empty() || new_trimmed.is_empty() {
                return Err(ApiError::bad_request("oldPath and newPath are required"));
            }
            let session = self.ensure_connected(&server_id).await?;
            session
                .sftp_rename(&old_trimmed, &new_trimmed)
                .await
                .map_err(|e| ApiError::internal(e.to_string()))?;
            let dir = parent_dir(&new_trimmed);
            self.emit_ui(ApiUiEvent::SwitchTab {
                server_id: server_id.clone(),
                tab: ApiUiTab::Sftp,
            });
            self.emit_ui(ApiUiEvent::AgentSftpRefresh {
                server_id,
                dir,
                status: format!("[MCP Agent] Renamed {old_trimmed} -> {new_trimmed}"),
            });
            Ok(())
        })
    }

    fn sftp_chmod(&self, server_id: String, path: String, mode: u32) -> BoxFuture<'_, ApiResult<()>> {
        Box::pin(async move {
            let trimmed = path.trim().to_string();
            if trimmed.is_empty() {
                return Err(ApiError::bad_request("path is required"));
            }
            if mode > 0o7777 {
                return Err(ApiError::bad_request("mode must be <= 07777"));
            }
            let session = self.ensure_connected(&server_id).await?;
            session
                .sftp_chmod(&trimmed, mode)
                .await
                .map_err(|e| ApiError::internal(e.to_string()))?;
            let dir = parent_dir(&trimmed);
            self.emit_ui(ApiUiEvent::SwitchTab {
                server_id: server_id.clone(),
                tab: ApiUiTab::Sftp,
            });
            self.emit_ui(ApiUiEvent::AgentSftpRefresh {
                server_id,
                dir,
                status: format!("[MCP Agent] chmod {mode:o} {trimmed}"),
            });
            Ok(())
        })
    }

    fn sftp_delete(
        &self,
        server_id: String,
        path: String,
        is_dir: bool,
    ) -> BoxFuture<'_, ApiResult<()>> {
        Box::pin(async move {
            let trimmed = path.trim().to_string();
            if trimmed.is_empty()
                || trimmed == "/"
                || trimmed == "."
                || trimmed == ".."
                || trimmed.trim_end_matches('/').is_empty()
            {
                return Err(ApiError::bad_request("Refusing to delete unsafe or root path"));
            }
            let session = self.ensure_connected(&server_id).await?;
            session
                .sftp_delete(&trimmed, is_dir)
                .await
                .map_err(|e| ApiError::internal(e.to_string()))?;
            let dir = parent_dir(&trimmed);
            self.emit_ui(ApiUiEvent::SwitchTab {
                server_id: server_id.clone(),
                tab: ApiUiTab::Sftp,
            });
            self.emit_ui(ApiUiEvent::AgentSftpRefresh {
                server_id,
                dir,
                status: format!("[MCP Agent] Deleted {trimmed}"),
            });
            Ok(())
        })
    }

    fn sftp_transfer(
        &self,
        source_server_id: String,
        source_path: String,
        dest_server_id: String,
        dest_path: String,
    ) -> BoxFuture<'_, ApiResult<u64>> {
        Box::pin(async move {
            let src_path = source_path.trim().to_string();
            let dst_path = dest_path.trim().to_string();
            if src_path.is_empty() || dst_path.is_empty() {
                return Err(ApiError::bad_request("sourcePath and destPath are required"));
            }
            let src_name = self.require_server_exists(&source_server_id)?;
            let _ = self.require_server_exists(&dest_server_id)?;
            let src_session = self.ensure_connected(&source_server_id).await?;
            let dst_session = self.ensure_connected(&dest_server_id).await?;
            let seq = XFER_SEQ.fetch_add(1, Ordering::Relaxed);
            let xfer_id = format!("mcp-xfer-{seq}");
            let bytes = sftp_transfer_between_sessions(
                &src_session,
                &src_path,
                &dst_session,
                &dst_path,
                xfer_id,
                None,
                None,
            )
            .await
            .map_err(|e| ApiError::internal(e.to_string()))?;
            let dir = parent_dir(&dst_path);
            self.emit_ui(ApiUiEvent::SwitchTab {
                server_id: dest_server_id.clone(),
                tab: ApiUiTab::Sftp,
            });
            self.emit_ui(ApiUiEvent::AgentSftpRefresh {
                server_id: dest_server_id,
                dir,
                status: format!(
                    "[MCP Agent] Transferred {bytes} bytes from {src_name}:{src_path} -> {dst_path}"
                ),
            });
            Ok(bytes)
        })
    }

    fn tunnels_list(&self, server_id: String) -> BoxFuture<'_, ApiResult<Vec<TunnelInfo>>> {
        Box::pin(async move {
            let session = self.ensure_connected(&server_id).await?;
            self.emit_ui(ApiUiEvent::SwitchTab {
                server_id: server_id.clone(),
                tab: ApiUiTab::Tunnels,
            });
            let list = session.list_tunnels().await;
            self.emit_ui(ApiUiEvent::TunnelsChanged { server_id });
            Ok(list)
        })
    }

    fn tunnels_start(
        &self,
        server_id: String,
        local_port: u16,
        remote_host: String,
        remote_port: u16,
        protocol: String,
    ) -> BoxFuture<'_, ApiResult<TunnelInfo>> {
        Box::pin(async move {
            if remote_port == 0 {
                return Err(ApiError::bad_request("remotePort must be non-zero"));
            }
            let session = self.ensure_connected(&server_id).await?;
            self.emit_ui(ApiUiEvent::SwitchTab {
                server_id: server_id.clone(),
                tab: ApiUiTab::Tunnels,
            });
            self.emit_ui(ApiUiEvent::AgentTunnelDirection {
                server_id: server_id.clone(),
                direction: "local".to_string(),
            });
            let info = session
                .start_local_tunnel(local_port, remote_host, remote_port, protocol)
                .await
                .map_err(|e| ApiError::internal(e.to_string()))?;
            self.emit_ui(ApiUiEvent::TunnelsChanged { server_id });
            Ok(info)
        })
    }

    fn tunnels_start_remote(
        &self,
        server_id: String,
        remote_bind_addr: String,
        remote_port: u16,
        local_host: String,
        local_port: u16,
    ) -> BoxFuture<'_, ApiResult<TunnelInfo>> {
        Box::pin(async move {
            if local_port == 0 {
                return Err(ApiError::bad_request("localPort must be non-zero"));
            }
            let session = self.ensure_connected(&server_id).await?;
            self.emit_ui(ApiUiEvent::SwitchTab {
                server_id: server_id.clone(),
                tab: ApiUiTab::Tunnels,
            });
            self.emit_ui(ApiUiEvent::AgentTunnelDirection {
                server_id: server_id.clone(),
                direction: "remote".to_string(),
            });
            let info = session
                .start_remote_tunnel(remote_bind_addr, remote_port, local_host, local_port)
                .await
                .map_err(|e| ApiError::internal(e.to_string()))?;
            self.emit_ui(ApiUiEvent::TunnelsChanged { server_id });
            Ok(info)
        })
    }

    fn tunnels_start_socks(&self, server_id: String, local_port: u16) -> BoxFuture<'_, ApiResult<TunnelInfo>> {
        Box::pin(async move {
            let session = self.ensure_connected(&server_id).await?;
            self.emit_ui(ApiUiEvent::SwitchTab {
                server_id: server_id.clone(),
                tab: ApiUiTab::Tunnels,
            });
            self.emit_ui(ApiUiEvent::AgentTunnelDirection {
                server_id: server_id.clone(),
                direction: "dynamic".to_string(),
            });
            let info = session
                .start_dynamic_socks5_tunnel(local_port)
                .await
                .map_err(|e| ApiError::internal(e.to_string()))?;
            self.emit_ui(ApiUiEvent::TunnelsChanged { server_id });
            Ok(info)
        })
    }

    fn tunnels_stop(
        &self,
        server_id: String,
        local_port: u16,
        tunnel_id: Option<String>,
    ) -> BoxFuture<'_, ApiResult<()>> {
        Box::pin(async move {
            let trimmed_id = tunnel_id.as_deref().map(str::trim).filter(|s| !s.is_empty());
            if trimmed_id.is_none() && local_port == 0 {
                return Err(ApiError::bad_request(
                    "Provide either tunnelId or a non-zero localPort",
                ));
            }
            let session = self.ensure_connected(&server_id).await?;
            self.emit_ui(ApiUiEvent::SwitchTab {
                server_id: server_id.clone(),
                tab: ApiUiTab::Tunnels,
            });
            if let Some(tid) = trimmed_id {
                session
                    .stop_tunnel_by_id(tid)
                    .await
                    .map_err(|e| ApiError::internal(e.to_string()))?;
            } else {
                session
                    .stop_tunnel(local_port)
                    .await
                    .map_err(|e| ApiError::internal(e.to_string()))?;
            }
            self.emit_ui(ApiUiEvent::TunnelsChanged { server_id });
            Ok(())
        })
    }

    fn secrets_list(&self, server_id: String) -> BoxFuture<'_, ApiResult<Vec<ApiSecretSummary>>> {
        Box::pin(async move {
            self.require_server_exists(&server_id)?;
            self.emit_ui(ApiUiEvent::SwitchTab {
                server_id: server_id.clone(),
                tab: ApiUiTab::Secrets,
            });
            let guard = self
                .vault
                .lock()
                .map_err(|_| ApiError::internal("Vault mutex poisoned"))?;
            Ok(guard
                .list_secrets(&server_id)
                .into_iter()
                .map(|s| ApiSecretSummary {
                    id: s.id,
                    name: s.name,
                })
                .collect())
        })
    }

    fn secrets_add(
        &self,
        server_id: String,
        name: String,
        value: String,
    ) -> BoxFuture<'_, ApiResult<String>> {
        Box::pin(async move {
            self.require_server_exists(&server_id)?;
            if name.trim().is_empty() {
                return Err(ApiError::bad_request("Secret name is required"));
            }
            self.emit_ui(ApiUiEvent::SwitchTab {
                server_id: server_id.clone(),
                tab: ApiUiTab::Secrets,
            });
            let id = {
                let mut guard = self
                    .vault
                    .lock()
                    .map_err(|_| ApiError::internal("Vault mutex poisoned"))?;
                guard
                    .add_secret(&server_id, name.trim(), &value)
                    .map_err(|e| ApiError::internal(e.to_string()))?
            };
            self.emit_ui(ApiUiEvent::SecretsChanged { server_id });
            Ok(id)
        })
    }

    fn containers_list(&self, server_id: String) -> BoxFuture<'_, ApiResult<String>> {
        Box::pin(async move {
            let session = self.ensure_connected(&server_id).await?;
            self.emit_ui(ApiUiEvent::SwitchTab {
                server_id,
                tab: ApiUiTab::Containers,
            });
            let (output, _) = session
                .list_containers()
                .await
                .map_err(|e| ApiError::internal(e.to_string()))?;
            Ok(output)
        })
    }

    fn containers_logs(
        &self,
        server_id: String,
        container_id: String,
        tail: u32,
    ) -> BoxFuture<'_, ApiResult<String>> {
        Box::pin(async move {
            let cid = container_id.trim().to_string();
            if cid.is_empty() {
                return Err(ApiError::bad_request("containerId is required"));
            }
            let session = self.ensure_connected(&server_id).await?;
            let logs = session
                .container_logs(&cid, tail)
                .await
                .map_err(|e| ApiError::bad_request(e.to_string()))?;
            self.emit_ui(ApiUiEvent::SwitchTab {
                server_id: server_id.clone(),
                tab: ApiUiTab::Containers,
            });
            self.emit_ui(ApiUiEvent::AgentContainerDetail {
                server_id,
                container_id: cid,
                show_inspect: false,
                logs: Some(logs.clone()),
                inspect: None,
            });
            Ok(logs)
        })
    }

    fn containers_inspect(
        &self,
        server_id: String,
        container_id: String,
        is_running: bool,
    ) -> BoxFuture<'_, ApiResult<ContainerDetails>> {
        Box::pin(async move {
            let cid = container_id.trim().to_string();
            if cid.is_empty() {
                return Err(ApiError::bad_request("containerId is required"));
            }
            let session = self.ensure_connected(&server_id).await?;
            let details = session
                .container_inspect(&cid, is_running)
                .await
                .map_err(|e| ApiError::bad_request(e.to_string()))?;
            self.emit_ui(ApiUiEvent::SwitchTab {
                server_id: server_id.clone(),
                tab: ApiUiTab::Containers,
            });
            self.emit_ui(ApiUiEvent::AgentContainerDetail {
                server_id,
                container_id: cid,
                show_inspect: true,
                logs: None,
                inspect: Some(Box::new(details.clone())),
            });
            Ok(details)
        })
    }

    fn containers_action(
        &self,
        server_id: String,
        container_id: String,
        action: String,
    ) -> BoxFuture<'_, ApiResult<()>> {
        Box::pin(async move {
            let cid = container_id.trim().to_string();
            if cid.is_empty() {
                return Err(ApiError::bad_request("containerId is required"));
            }
            let act = parse_container_action(&action)?;
            let session = self.ensure_connected(&server_id).await?;
            session
                .container_action(&cid, act)
                .await
                .map_err(|e| ApiError::internal(e.to_string()))?;
            self.emit_ui(ApiUiEvent::SwitchTab {
                server_id,
                tab: ApiUiTab::Containers,
            });
            Ok(())
        })
    }

    fn containers_compose(
        &self,
        server_id: String,
        dir: String,
        action: String,
    ) -> BoxFuture<'_, ApiResult<String>> {
        Box::pin(async move {
            let trimmed_dir = dir.trim().to_string();
            if trimmed_dir.is_empty() {
                return Err(ApiError::bad_request("dir is required"));
            }
            let up = match action.trim().to_ascii_lowercase().as_str() {
                "up" => true,
                "down" => false,
                other => {
                    return Err(ApiError::bad_request(format!(
                        "Invalid compose action \"{other}\" (expected up or down)"
                    )))
                }
            };
            let session = self.ensure_connected(&server_id).await?;
            let output = session
                .compose_action(&trimmed_dir, up)
                .await
                .map_err(|e| ApiError::internal(e.to_string()))?;
            self.emit_ui(ApiUiEvent::SwitchTab {
                server_id: server_id.clone(),
                tab: ApiUiTab::Containers,
            });
            self.emit_ui(ApiUiEvent::AgentComposeResult {
                server_id,
                dir: trimmed_dir,
                output: output.clone(),
            });
            Ok(output)
        })
    }

    fn commands_list(&self, server_id: String) -> BoxFuture<'_, ApiResult<Vec<CommandSnippet>>> {
        Box::pin(async move {
            self.require_server_exists(&server_id)?;
            self.emit_ui(ApiUiEvent::SwitchTab {
                server_id: server_id.clone(),
                tab: ApiUiTab::Commands,
            });
            let guard = self
                .vault
                .lock()
                .map_err(|_| ApiError::internal("Vault mutex poisoned"))?;
            Ok(guard.list_commands(&server_id))
        })
    }

    fn commands_add(
        &self,
        server_id: String,
        name: String,
        cmd: String,
    ) -> BoxFuture<'_, ApiResult<u64>> {
        Box::pin(async move {
            self.require_server_exists(&server_id)?;
            if name.trim().is_empty() || cmd.trim().is_empty() {
                return Err(ApiError::bad_request("Command name and cmd are required"));
            }
            self.emit_ui(ApiUiEvent::SwitchTab {
                server_id: server_id.clone(),
                tab: ApiUiTab::Commands,
            });
            let id = {
                let mut guard = self
                    .vault
                    .lock()
                    .map_err(|_| ApiError::internal("Vault mutex poisoned"))?;
                guard
                    .add_command(&server_id, name.trim(), cmd.trim())
                    .map_err(|e| ApiError::internal(e.to_string()))?
            };
            self.emit_ui(ApiUiEvent::CommandsChanged { server_id });
            Ok(id)
        })
    }

    fn commands_remove(&self, server_id: String, id: u64) -> BoxFuture<'_, ApiResult<()>> {
        Box::pin(async move {
            self.require_server_exists(&server_id)?;
            self.emit_ui(ApiUiEvent::SwitchTab {
                server_id: server_id.clone(),
                tab: ApiUiTab::Commands,
            });
            {
                let mut guard = self
                    .vault
                    .lock()
                    .map_err(|_| ApiError::internal("Vault mutex poisoned"))?;
                guard
                    .remove_command(&server_id, id)
                    .map_err(|e| ApiError::internal(e.to_string()))?;
            }
            self.emit_ui(ApiUiEvent::CommandsChanged { server_id });
            Ok(())
        })
    }

    fn groups_list(&self) -> BoxFuture<'_, ApiResult<Vec<ApiGroupSummary>>> {
        Box::pin(async move {
            let guard = self
                .vault
                .lock()
                .map_err(|_| ApiError::internal("Vault mutex poisoned"))?;
            let servers = guard.list_servers();
            Ok(guard
                .list_groups()
                .iter()
                .map(|g| {
                    let server_ids = servers
                        .iter()
                        .filter(|s| guard.is_server_in_group_subtree(s, &g.id))
                        .map(|s| s.id.clone())
                        .collect();
                    ApiGroupSummary {
                        id: g.id.clone(),
                        name: g.name.clone(),
                        parent_id: g.parent_id.clone(),
                        tags: g.tags.clone(),
                        server_ids,
                    }
                })
                .collect())
        })
    }

    fn groups_execute(
        &self,
        group_id: String,
        command: String,
    ) -> BoxFuture<'_, ApiResult<Vec<GroupCommandResult>>> {
        Box::pin(async move {
            let gid = group_id.trim().to_string();
            let cmd = command.trim().to_string();
            if gid.is_empty() || cmd.is_empty() {
                return Err(ApiError::bad_request("groupId and command are required"));
            }
            let targets: Vec<(String, String)> = {
                let guard = self
                    .vault
                    .lock()
                    .map_err(|_| ApiError::internal("Vault mutex poisoned"))?;
                if !guard.list_groups().iter().any(|g| g.id == gid) {
                    return Err(ApiError::not_found(format!("Group \"{gid}\" not found")));
                }
                guard
                    .list_servers()
                    .into_iter()
                    .filter(|s| guard.is_server_in_group_subtree(s, &gid))
                    .map(|s| (s.id, s.name))
                    .collect()
            };

            let mut results = Vec::with_capacity(targets.len());
            for (server_id, server_name) in targets {
                match self.execute(server_id.clone(), cmd.clone()).await {
                    Ok(output) => results.push(GroupCommandResult {
                        server_id,
                        server_name,
                        ok: true,
                        output: Some(output),
                        error: None,
                    }),
                    Err(err) => results.push(GroupCommandResult {
                        server_id,
                        server_name,
                        ok: false,
                        output: None,
                        error: Some(err.message),
                    }),
                }
            }
            Ok(results)
        })
    }

    fn snippet_packages_list(&self) -> BoxFuture<'_, ApiResult<Vec<SnippetPackage>>> {
        Box::pin(async move {
            let guard = self
                .vault
                .lock()
                .map_err(|_| ApiError::internal("Vault mutex poisoned"))?;
            Ok(guard.list_snippet_packages().to_vec())
        })
    }

    fn snippet_packages_add(
        &self,
        name: String,
        description: String,
        snippets: Vec<SnippetInput>,
    ) -> BoxFuture<'_, ApiResult<SnippetPackage>> {
        Box::pin(async move {
            let trimmed_name = name.trim().to_string();
            if trimmed_name.is_empty() {
                return Err(ApiError::bad_request("Package name is required"));
            }
            for s in &snippets {
                if s.name.trim().is_empty() || s.cmd.trim().is_empty() {
                    return Err(ApiError::bad_request(
                        "Each snippet requires non-empty name and cmd",
                    ));
                }
            }
            let created = {
                let mut guard = self
                    .vault
                    .lock()
                    .map_err(|_| ApiError::internal("Vault mutex poisoned"))?;
                let pkg_id = guard
                    .add_snippet_package(&trimmed_name, description.trim())
                    .map_err(|e| ApiError::internal(e.to_string()))?;
                for s in snippets {
                    guard
                        .add_package_snippet(&pkg_id, s.name.trim(), s.cmd.trim())
                        .map_err(|e| ApiError::internal(e.to_string()))?;
                }
                guard
                    .list_snippet_packages()
                    .iter()
                    .find(|p| p.id == pkg_id)
                    .cloned()
                    .ok_or_else(|| ApiError::internal("Created snippet package missing"))?
            };
            self.emit_ui(ApiUiEvent::VaultChanged);
            Ok(created)
        })
    }

    fn ssh_keys_list(&self) -> BoxFuture<'_, ApiResult<Vec<SshKeySummary>>> {
        Box::pin(async move {
            let guard = self
                .vault
                .lock()
                .map_err(|_| ApiError::internal("Vault mutex poisoned"))?;
            Ok(guard.list_ssh_keys())
        })
    }

    fn identities_list(&self) -> BoxFuture<'_, ApiResult<Vec<IdentitySummary>>> {
        Box::pin(async move {
            let guard = self
                .vault
                .lock()
                .map_err(|_| ApiError::internal("Vault mutex poisoned"))?;
            Ok(guard.list_identities())
        })
    }

    fn known_hosts_list(&self) -> BoxFuture<'_, ApiResult<Vec<KnownHostEntry>>> {
        Box::pin(async move {
            let guard = self
                .vault
                .lock()
                .map_err(|_| ApiError::internal("Vault mutex poisoned"))?;
            Ok(guard.list_known_hosts().to_vec())
        })
    }

    fn vnc_open(
        &self,
        server_id: String,
        host: String,
        port: u16,
        auto_connect: bool,
    ) -> BoxFuture<'_, ApiResult<()>> {
        Box::pin(async move {
            if port == 0 {
                return Err(ApiError::bad_request("port must be non-zero"));
            }
            let target_host = if host.trim().is_empty() {
                "127.0.0.1".to_string()
            } else {
                host.trim().to_string()
            };
            let _session = self.ensure_connected(&server_id).await?;
            self.emit_ui(ApiUiEvent::SwitchTab {
                server_id: server_id.clone(),
                tab: ApiUiTab::Vnc,
            });
            self.emit_ui(ApiUiEvent::AgentOpenVnc {
                server_id,
                host: target_host,
                port,
                auto_connect,
            });
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_token_created_once_and_reused() {
        let dir = std::env::temp_dir().join(format!("glyph-api-token-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("mcp_token");

        let t1 = ensure_mcp_token(&path).expect("create token");
        assert_eq!(t1.len(), 64);
        assert!(t1.chars().all(|c| c.is_ascii_hexdigit()));

        let t2 = ensure_mcp_token(&path).expect("reuse token");
        assert_eq!(t1, t2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn constant_time_eq_checks_exact_match() {
        assert!(constant_time_eq(b"abc123", b"abc123"));
        assert!(!constant_time_eq(b"abc123", b"abc124"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
        assert!(!constant_time_eq(b"", b""));
    }
}
