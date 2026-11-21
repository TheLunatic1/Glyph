use anyhow::{anyhow, Context, Result};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use directories::BaseDirs;
use glyph_core::crypto::write_restricted_file;
use glyph_core::models::{CommandSnippet, ConnectionConfig};
use glyph_core::ssh::{ConnectOutcome, HostKeyPrompt, SessionEvent, SshSession};
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
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::task::JoinHandle;

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
    AgentCommandStarted {
        server_id: String,
        command: String,
    },
    AgentCommandOutput {
        server_id: String,
        output: String,
    },
    AgentSftpFile {
        server_id: String,
        action: String,
        path: String,
        content: String,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApiSecretSummary {
    pub id: String,
    pub name: String,
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
    fn execute(&self, server_id: String, command: String) -> BoxFuture<'_, ApiResult<String>>;
    fn sftp_read(&self, server_id: String, path: String) -> BoxFuture<'_, ApiResult<String>>;
    fn sftp_write(
        &self,
        server_id: String,
        path: String,
        content: String,
    ) -> BoxFuture<'_, ApiResult<()>>;
    fn tunnels_start(
        &self,
        server_id: String,
        local_port: u16,
        remote_host: String,
        remote_port: u16,
        protocol: String,
    ) -> BoxFuture<'_, ApiResult<TunnelInfo>>;
    fn tunnels_stop(&self, server_id: String, local_port: u16) -> BoxFuture<'_, ApiResult<()>>;
    fn secrets_list(&self, server_id: String) -> BoxFuture<'_, ApiResult<Vec<ApiSecretSummary>>>;
    fn secrets_add(
        &self,
        server_id: String,
        name: String,
        value: String,
    ) -> BoxFuture<'_, ApiResult<String>>;
    fn containers_list(&self, server_id: String) -> BoxFuture<'_, ApiResult<String>>;
    fn commands_list(&self, server_id: String) -> BoxFuture<'_, ApiResult<Vec<CommandSnippet>>>;
    fn commands_add(
        &self,
        server_id: String,
        name: String,
        cmd: String,
    ) -> BoxFuture<'_, ApiResult<u64>>;
    fn commands_remove(&self, server_id: String, id: u64) -> BoxFuture<'_, ApiResult<()>>;
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
        .route("/api/execute", post(handle_execute))
        .route("/api/sftp/read", post(handle_sftp_read))
        .route("/api/sftp/write", post(handle_sftp_write))
        .route("/api/tunnels/start", post(handle_tunnels_start))
        .route("/api/tunnels/stop", post(handle_tunnels_stop))
        .route("/api/secrets/list", post(handle_secrets_list))
        .route("/api/secrets/add", post(handle_secrets_add))
        .route("/api/containers/list", post(handle_containers_list))
        .route("/api/commands/list", post(handle_commands_list))
        .route("/api/commands/add", post(handle_commands_add))
        .route("/api/commands/remove", post(handle_commands_remove))
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
    local_port: u16,
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
        .tunnels_stop(body.server_id, body.local_port)
        .await?;
    Ok(Json(json!({ "success": true })))
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
) -> ApiResult<Json<Vec<CommandSnippet>>> {
    let list = state.controller.commands_list(body.server_id).await?;
    Ok(Json(list))
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
            let info = session
                .start_local_tunnel(local_port, remote_host, remote_port, protocol)
                .await
                .map_err(|e| ApiError::internal(e.to_string()))?;
            self.emit_ui(ApiUiEvent::TunnelsChanged { server_id });
            Ok(info)
        })
    }

    fn tunnels_stop(&self, server_id: String, local_port: u16) -> BoxFuture<'_, ApiResult<()>> {
        Box::pin(async move {
            if local_port == 0 {
                return Err(ApiError::bad_request("localPort must be non-zero"));
            }
            let session = self.ensure_connected(&server_id).await?;
            self.emit_ui(ApiUiEvent::SwitchTab {
                server_id: server_id.clone(),
                tab: ApiUiTab::Tunnels,
            });
            session
                .stop_tunnel(local_port)
                .await
                .map_err(|e| ApiError::internal(e.to_string()))?;
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
