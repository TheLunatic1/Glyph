use crate::models::{AuthMethod, ConnectionConfig, HostKeyStatus, TelemetrySnapshot};
use crate::telemetry::{TelemetryTracker, GLYPH_STAT_CMD};
use anyhow::{anyhow, Context, Result};
use futures::Future;
use russh::client::{self, DisconnectReason, KeyboardInteractiveAuthResponse};
use russh::keys::agent::client::AgentClient;
use russh::keys::{HashAlg, PrivateKey, PrivateKeyWithHashAlg};
use russh::{ChannelMsg, Disconnect};
use std::collections::HashMap;
use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, Mutex, Semaphore};

const MAX_CONCURRENT_EXEC: usize = 4;

#[derive(Debug)]
pub struct HostKeyPrompt {
    pub host: String,
    pub port: u16,
    pub algorithm: String,
    pub fingerprint_sha256: String,
    pub mismatch_expected: Option<String>,
    pub responder: oneshot::Sender<bool>,
}

#[derive(Debug, Clone)]
pub enum ShellEvent {
    Data { tab_id: String, bytes: Vec<u8> },
    Closed { tab_id: String },
}

#[derive(Debug, Clone)]
pub enum SessionEvent {
    Status(String),
    Stats(TelemetrySnapshot),
    Shell(ShellEvent),
    Disconnected(String),
}

#[derive(Debug, Clone)]
pub struct TrustedHostRecord {
    pub host: String,
    pub port: u16,
    pub algorithm: String,
    pub fingerprint_sha256: String,
}

#[derive(Clone)]
pub struct ShellHandle {
    cmd_tx: mpsc::UnboundedSender<ShellCmd>,
}

enum ShellCmd {
    Data(Vec<u8>),
    Resize(u32, u32),
    Close,
}

impl ShellHandle {
    pub fn write(&self, bytes: Vec<u8>) {
        let _ = self.cmd_tx.send(ShellCmd::Data(bytes));
    }

    pub fn resize(&self, cols: u32, rows: u32) {
        let _ = self.cmd_tx.send(ShellCmd::Resize(cols, rows));
    }

    pub fn close(&self) {
        let _ = self.cmd_tx.send(ShellCmd::Close);
    }
}

pub struct GlyphHandler {
    host: String,
    port: u16,
    known_hosts: Arc<HashMap<String, String>>,
    host_key_tx: mpsc::UnboundedSender<HostKeyPrompt>,
    newly_trusted: Arc<Mutex<Vec<TrustedHostRecord>>>,
    disconnect_tx: Option<mpsc::UnboundedSender<SessionEvent>>,
    intentional_close: Arc<AtomicBool>,
}

impl client::Handler for GlyphHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::PublicKeyOrCertificate,
    ) -> std::result::Result<bool, Self::Error> {
        let pk = server_public_key.public_key();
        let fp = pk.fingerprint(HashAlg::Sha256).to_string();
        let alg = pk.algorithm().to_string();
        let key = format!("{}:{}", self.host, self.port);

        let status = match self.known_hosts.get(&key) {
            None => HostKeyStatus::Unknown,
            Some(expected) if expected == &fp => HostKeyStatus::Trusted,
            Some(expected) => HostKeyStatus::Mismatch {
                expected: expected.clone(),
                actual: fp.clone(),
            },
        };

        if status == HostKeyStatus::Trusted {
            return Ok(true);
        }

        let mismatch_expected = match status {
            HostKeyStatus::Mismatch { expected, .. } => Some(expected),
            _ => None,
        };

        let (resp_tx, resp_rx) = oneshot::channel();
        let prompt = HostKeyPrompt {
            host: self.host.clone(),
            port: self.port,
            algorithm: alg.clone(),
            fingerprint_sha256: fp.clone(),
            mismatch_expected,
            responder: resp_tx,
        };

        if self.host_key_tx.send(prompt).is_err() {
            return Ok(false);
        }

        let accepted = resp_rx.await.unwrap_or(false);
        if accepted {
            self.newly_trusted.lock().await.push(TrustedHostRecord {
                host: self.host.clone(),
                port: self.port,
                algorithm: alg,
                fingerprint_sha256: fp,
            });
        }
        Ok(accepted)
    }

    async fn disconnected(
        &mut self,
        reason: DisconnectReason<Self::Error>,
    ) -> std::result::Result<(), Self::Error> {
        if !self.intentional_close.load(Ordering::SeqCst) {
            if let Some(tx) = &self.disconnect_tx {
                let msg = match &reason {
                    DisconnectReason::ReceivedDisconnect(info) => {
                        if info.message.is_empty() {
                            "Connection closed by remote server.".to_string()
                        } else {
                            format!("Disconnected: {}", info.message)
                        }
                    }
                    DisconnectReason::Error(e) => format!("Connection was lost: {e}"),
                };
                let _ = tx.send(SessionEvent::Disconnected(msg));
            }
        }
        match reason {
            DisconnectReason::ReceivedDisconnect(_) => Ok(()),
            DisconnectReason::Error(e) => Err(e),
        }
    }
}

#[derive(Clone)]
pub struct SshSession {
    inner: Arc<SshSessionInner>,
}

struct SshSessionInner {
    handle: Arc<Mutex<Option<client::Handle<GlyphHandler>>>>,
    exec_sem: Arc<Semaphore>,
    shells: Mutex<HashMap<String, ShellHandle>>,
    stat_abort: Mutex<Option<tokio::task::JoinHandle<()>>>,
    intentional_close: Arc<AtomicBool>,
}

pub struct ConnectOutcome {
    pub session: SshSession,
    pub os: Option<String>,
    pub newly_trusted_hosts: Vec<TrustedHostRecord>,
}

impl SshSession {
    pub async fn connect(
        config: ConnectionConfig,
        known_hosts: HashMap<String, String>,
        event_tx: mpsc::UnboundedSender<SessionEvent>,
        host_key_tx: mpsc::UnboundedSender<HostKeyPrompt>,
    ) -> Result<ConnectOutcome> {
        if !config.zerotier.trim().is_empty() {
            let _ = event_tx.send(SessionEvent::Status(format!(
                "Note: Built-in ZeroTier node is deferred in v3 alpha; connecting to {}:{} over system routing.",
                config.host, config.port
            )));
        }

        let known_hosts = Arc::new(known_hosts);
        let newly_trusted = Arc::new(Mutex::new(Vec::new()));
        let intentional_close = Arc::new(AtomicBool::new(false));

        let handle = connect_recursive(
            &config,
            Arc::clone(&known_hosts),
            Arc::clone(&newly_trusted),
            Arc::clone(&intentional_close),
            event_tx.clone(),
            host_key_tx,
            true,
        )
        .await?;

        let inner = Arc::new(SshSessionInner {
            handle: Arc::new(Mutex::new(Some(handle))),
            exec_sem: Arc::new(Semaphore::new(MAX_CONCURRENT_EXEC)),
            shells: Mutex::new(HashMap::new()),
            stat_abort: Mutex::new(None),
            intentional_close,
        });

        let session = SshSession { inner };

        let _ = event_tx.send(SessionEvent::Status("Detecting remote OS...".to_string()));
        let os = session.detect_os().await;

        let trusted_records = newly_trusted.lock().await.clone();

        Ok(ConnectOutcome {
            session,
            os,
            newly_trusted_hosts: trusted_records,
        })
    }

    pub fn start_stat_polling(&self, event_tx: mpsc::UnboundedSender<SessionEvent>) {
        let session = self.clone();
        let inner = Arc::clone(&self.inner);
        tokio::spawn(async move {
            let mut prev = inner.stat_abort.lock().await;
            if let Some(old) = prev.take() {
                old.abort();
            }
            let handle = tokio::spawn(async move {
                let mut tracker = TelemetryTracker::new();
                let mut ticker = tokio::time::interval(Duration::from_secs(3));
                loop {
                    ticker.tick().await;
                    if !session.is_connected().await {
                        break;
                    }
                    if let Ok(raw) = session.exec(GLYPH_STAT_CMD).await {
                        let snap = tracker.parse(&raw);
                        if event_tx.send(SessionEvent::Stats(snap)).is_err() {
                            break;
                        }
                    }
                }
            });
            *prev = Some(handle);
        });
    }

    pub async fn is_connected(&self) -> bool {
        let guard = self.inner.handle.lock().await;
        guard.as_ref().is_some_and(|h| !h.is_closed())
    }

    pub async fn exec(&self, command: &str) -> Result<String> {
        let _permit = self
            .inner
            .exec_sem
            .acquire()
            .await
            .map_err(|_| anyhow!("Session closed"))?;

        let mut channel = {
            let guard = self.inner.handle.lock().await;
            let handle = guard
                .as_ref()
                .ok_or_else(|| anyhow!("Not connected"))?;
            handle
                .channel_open_session()
                .await
                .context("Failed to open SSH exec channel")?
        };

        channel
            .exec(true, command.as_bytes())
            .await
            .context("Failed to execute remote command")?;

        let mut output = Vec::new();
        while let Some(msg) = channel.wait().await {
            match msg {
                ChannelMsg::Data { data } => output.extend_from_slice(&data),
                ChannelMsg::ExtendedData { data, .. } => output.extend_from_slice(&data),
                ChannelMsg::Close | ChannelMsg::Eof => break,
                _ => {}
            }
        }
        let _ = channel.close().await;
        Ok(String::from_utf8_lossy(&output).into_owned())
    }

    pub async fn detect_os(&self) -> Option<String> {
        let cmd = concat!(
            r#"if [ -f /etc/openwrt_release ]; then echo openwrt; "#,
            r#"elif [ -f /etc/os-release ]; then grep "^ID=" /etc/os-release 2>/dev/null | head -1 | cut -d= -f2 | tr -d '"'; "#,
            r#"elif [ -f /etc/alpine-release ]; then echo alpine; "#,
            r#"elif [ -f /etc/gentoo-release ]; then echo gentoo; "#,
            r#"elif [ -f /etc/freebsd-version ] || uname -s 2>/dev/null | grep -qi freebsd; then echo freebsd; "#,
            r#"else uname -s 2>/dev/null | tr "[:upper:]" "[:lower:]"; fi 2>/dev/null"#,
        );
        match self.exec(cmd).await {
            Ok(raw) => raw
                .lines()
                .next()
                .map(|s| s.trim().to_ascii_lowercase())
                .filter(|s| !s.is_empty()),
            Err(_) => None,
        }
    }

    pub async fn deploy_public_key(&self, public_key: &str) -> Result<()> {
        let trimmed = public_key.trim();
        if trimmed.is_empty() || trimmed.contains('\n') || trimmed.contains('\r') {
            return Err(anyhow!("Invalid OpenSSH public key"));
        }
        let escaped = trimmed.replace('\'', "'\\''");
        let cmd = format!(
            "mkdir -p ~/.ssh && chmod 700 ~/.ssh && touch ~/.ssh/authorized_keys && chmod 600 ~/.ssh/authorized_keys && (grep -qxF '{escaped}' ~/.ssh/authorized_keys || printf '%s\\n' '{escaped}' >> ~/.ssh/authorized_keys)"
        );
        self.exec(&cmd).await?;
        Ok(())
    }

    pub async fn open_shell(
        &self,
        tab_id: String,
        cols: u32,
        rows: u32,
        event_tx: mpsc::UnboundedSender<SessionEvent>,
    ) -> Result<ShellHandle> {
        let channel = {
            let guard = self.inner.handle.lock().await;
            let handle = guard
                .as_ref()
                .ok_or_else(|| anyhow!("Not connected"))?;
            handle
                .channel_open_session()
                .await
                .context("Failed to open PTY session channel")?
        };

        let (mut rx, tx) = channel.split();
        tx.request_pty(false, "xterm-256color", cols.max(10), rows.max(4), 0, 0, &[])
            .await
            .context("PTY allocation failed")?;
        tx.request_shell(false)
            .await
            .context("Failed to start remote shell")?;

        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<ShellCmd>();
        let shell_handle = ShellHandle { cmd_tx };

        {
            let mut shells = self.inner.shells.lock().await;
            if let Some(old) = shells.insert(tab_id.clone(), shell_handle.clone()) {
                old.close();
            }
        }

        let tab_read = tab_id.clone();
        let tx_read = event_tx.clone();
        let inner_weak = Arc::downgrade(&self.inner);

        // Writer + Reader coordinated task
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    biased;
                    cmd = cmd_rx.recv() => {
                        match cmd {
                            Some(ShellCmd::Data(bytes)) => {
                                if tx.data_bytes(bytes).await.is_err() {
                                    break;
                                }
                            }
                            Some(ShellCmd::Resize(c, r)) => {
                                let _ = tx.window_change(c.max(10), r.max(4), 0, 0).await;
                            }
                            Some(ShellCmd::Close) | None => {
                                let _ = tx.eof().await;
                                let _ = tx.close().await;
                                break;
                            }
                        }
                    }
                    msg = rx.wait() => {
                        match msg {
                            Some(ChannelMsg::Data { data }) => {
                                let _ = tx_read.send(SessionEvent::Shell(ShellEvent::Data {
                                    tab_id: tab_read.clone(),
                                    bytes: data.to_vec(),
                                }));
                            }
                            Some(ChannelMsg::ExtendedData { data, .. }) => {
                                let _ = tx_read.send(SessionEvent::Shell(ShellEvent::Data {
                                    tab_id: tab_read.clone(),
                                    bytes: data.to_vec(),
                                }));
                            }
                            Some(ChannelMsg::Close | ChannelMsg::Eof) | None => {
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
            if let Some(inner) = inner_weak.upgrade() {
                inner.shells.lock().await.remove(&tab_read);
            }
            let _ = tx_read.send(SessionEvent::Shell(ShellEvent::Closed {
                tab_id: tab_read,
            }));
        });

        Ok(shell_handle)
    }

    pub async fn close_shell(&self, tab_id: &str) {
        if let Some(shell) = self.inner.shells.lock().await.remove(tab_id) {
            shell.close();
        }
    }

    pub async fn disconnect(&self) {
        self.inner.intentional_close.store(true, Ordering::SeqCst);

        if let Some(h) = self.inner.stat_abort.lock().await.take() {
            h.abort();
        }

        let shells: Vec<ShellHandle> = {
            let mut map = self.inner.shells.lock().await;
            map.drain().map(|(_, v)| v).collect()
        };
        for sh in shells {
            sh.close();
        }

        if let Some(handle) = self.inner.handle.lock().await.take() {
            let _ = handle
                .disconnect(Disconnect::ByApplication, "User disconnected", "en")
                .await;
        }
    }
}

fn connect_recursive<'a>(
    config: &'a ConnectionConfig,
    known_hosts: Arc<HashMap<String, String>>,
    newly_trusted: Arc<Mutex<Vec<TrustedHostRecord>>>,
    intentional_close: Arc<AtomicBool>,
    event_tx: mpsc::UnboundedSender<SessionEvent>,
    host_key_tx: mpsc::UnboundedSender<HostKeyPrompt>,
    is_target: bool,
) -> Pin<Box<dyn Future<Output = Result<client::Handle<GlyphHandler>>> + Send + 'a>> {
    Box::pin(async move {
        let russh_cfg = Arc::new(client::Config {
            inactivity_timeout: Some(Duration::from_secs(300)),
            keepalive_interval: Some(Duration::from_secs(15)),
            keepalive_max: 3,
            ..Default::default()
        });

        let mut handle = if let Some(jump_cfg) = &config.proxy_jump {
            let _ = event_tx.send(SessionEvent::Status(format!(
                "Connecting via ProxyJump {}@{}:{}...",
                jump_cfg.username, jump_cfg.host, jump_cfg.port
            )));
            let jump_handle = connect_recursive(
                jump_cfg,
                Arc::clone(&known_hosts),
                Arc::clone(&newly_trusted),
                Arc::clone(&intentional_close),
                event_tx.clone(),
                host_key_tx.clone(),
                false,
            )
            .await?;

            let _ = event_tx.send(SessionEvent::Status(format!(
                "Opening forward channel through jump host to {}:{}...",
                config.host, config.port
            )));
            let channel = jump_handle
                .channel_open_direct_tcpip(&config.host, config.port as u32, "127.0.0.1", 0)
                .await
                .context("ProxyJump direct-tcpip failed")?;
            let stream = channel.into_stream();

            let handler = GlyphHandler {
                host: config.host.clone(),
                port: config.port,
                known_hosts,
                host_key_tx,
                newly_trusted,
                disconnect_tx: if is_target {
                    Some(event_tx.clone())
                } else {
                    None
                },
                intentional_close,
            };
            tokio::time::timeout(
                Duration::from_secs(15),
                client::connect_stream(russh_cfg, stream, handler),
            )
            .await
            .map_err(|_| anyhow!("SSH handshake timed out for {}:{}", config.host, config.port))?
            .map_err(|e| anyhow!("SSH connection failed: {e}"))?
        } else {
            let _ = event_tx.send(SessionEvent::Status(format!(
                "Initializing connection to {}:{}...",
                config.host, config.port
            )));
            let addr = format!("{}:{}", config.host, config.port);
            let handler = GlyphHandler {
                host: config.host.clone(),
                port: config.port,
                known_hosts,
                host_key_tx,
                newly_trusted,
                disconnect_tx: if is_target {
                    Some(event_tx.clone())
                } else {
                    None
                },
                intentional_close,
            };
            tokio::time::timeout(
                Duration::from_secs(15),
                client::connect(russh_cfg, addr, handler),
            )
            .await
            .map_err(|_| anyhow!("Connection timed out to {}:{}", config.host, config.port))?
            .map_err(|e| anyhow!("SSH connection failed: {e}"))?
        };

        let _ = event_tx.send(SessionEvent::Status(format!(
            "Authenticating as {}...",
            config.username
        )));
        authenticate_session(&mut handle, config, &event_tx).await?;
        Ok(handle)
    })
}

async fn authenticate_session(
    handle: &mut client::Handle<GlyphHandler>,
    config: &ConnectionConfig,
    event_tx: &mpsc::UnboundedSender<SessionEvent>,
) -> Result<()> {
    let user = &config.username;
    let pass = config.passphrase.as_deref().filter(|s| !s.is_empty());

    // 1. Vault key
    if let Some(pem) = &config.private_key_pem {
        let _ = event_tx.send(SessionEvent::Status(
            "Trying Vault SSH private key...".to_string(),
        ));
        let key = decode_private_key(pem, pass)?;
        if try_public_key_auth(handle, user, key).await? {
            return Ok(());
        }
    }

    // 2. Key file path
    if let Some(path) = &config.private_key_path {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            let _ = event_tx.send(SessionEvent::Status(format!(
                "Loading SSH private key from {}...",
                trimmed
            )));
            let pem = tokio::fs::read_to_string(Path::new(trimmed))
                .await
                .with_context(|| format!("Could not read private key at \"{trimmed}\""))?;
            let key = decode_private_key(&pem, pass)?;
            if try_public_key_auth(handle, user, key).await? {
                return Ok(());
            }
        }
    }

    // 3. SSH Agent (when requested or as candidate)
    if config.auth_method == AuthMethod::Agent {
        let _ = event_tx.send(SessionEvent::Status(
            "Querying local SSH Agent...".to_string(),
        ));
        if try_agent_auth(handle, user).await? {
            return Ok(());
        }
    }

    // 4. Password
    if let Some(password) = config.password.as_deref().filter(|p| !p.is_empty()) {
        let _ = event_tx.send(SessionEvent::Status(
            "Authenticating with password...".to_string(),
        ));
        let res = handle
            .authenticate_password(user, password)
            .await
            .map_err(|e| anyhow!("Password auth error: {e}"))?;
        if res.success() {
            return Ok(());
        }

        // Keyboard-interactive fallback for PAM / challenge-response servers
        if try_keyboard_interactive(handle, user, password).await? {
            return Ok(());
        }
    }

    // 5. Try SSH Agent as last-resort fallback if user didn't specify password/key
    if config.auth_method != AuthMethod::Agent && try_agent_auth(handle, user).await.unwrap_or(false)
    {
        return Ok(());
    }

    Err(anyhow!("All configured authentication methods were rejected by the server."))
}

fn decode_private_key(pem: &str, passphrase: Option<&str>) -> Result<PrivateKey> {
    let key = PrivateKey::from_openssh(pem.trim())
        .map_err(|e| anyhow!("Invalid SSH private key format: {e}"))?;
    if key.is_encrypted() {
        let pass = passphrase.ok_or_else(|| {
            anyhow!("Private key is passphrase-protected, but no passphrase was provided")
        })?;
        key.decrypt(pass)
            .map_err(|_| anyhow!("Incorrect passphrase for SSH private key"))
    } else {
        Ok(key)
    }
}

async fn try_public_key_auth(
    handle: &mut client::Handle<GlyphHandler>,
    user: &str,
    key: PrivateKey,
) -> Result<bool> {
    let hash_alg = if matches!(key.algorithm(), russh::keys::Algorithm::Rsa { .. }) {
        handle
            .best_supported_rsa_hash()
            .await
            .ok()
            .flatten()
            .flatten()
    } else {
        None
    };

    let res = handle
        .authenticate_publickey(user, PrivateKeyWithHashAlg::new(Arc::new(key), hash_alg))
        .await
        .map_err(|e| anyhow!("Public key auth error: {e}"))?;
    Ok(res.success())
}

async fn try_keyboard_interactive(
    handle: &mut client::Handle<GlyphHandler>,
    user: &str,
    password: &str,
) -> Result<bool> {
    let mut resp = handle
        .authenticate_keyboard_interactive_start(user, None)
        .await
        .map_err(|e| anyhow!("Keyboard-interactive error: {e}"))?;
    for _ in 0..4 {
        match resp {
            KeyboardInteractiveAuthResponse::Success => return Ok(true),
            KeyboardInteractiveAuthResponse::Failure { .. } => return Ok(false),
            KeyboardInteractiveAuthResponse::InfoRequest { prompts, .. } => {
                let answers = prompts.into_iter().map(|_| password.to_string()).collect();
                resp = handle
                    .authenticate_keyboard_interactive_respond(answers)
                    .await
                    .map_err(|e| anyhow!("Keyboard-interactive response error: {e}"))?;
            }
        }
    }
    Ok(false)
}

async fn try_agent_auth(handle: &mut client::Handle<GlyphHandler>, user: &str) -> Result<bool> {
    #[cfg(unix)]
    let mut agent = match AgentClient::connect_env().await {
        Ok(a) => a.dynamic(),
        Err(_) => return Ok(false),
    };

    #[cfg(windows)]
    let mut agent = match AgentClient::connect_named_pipe(r"\\.\pipe\openssh-ssh-agent").await {
        Ok(a) => a.dynamic(),
        Err(_) => match AgentClient::connect_pageant().await {
            Ok(p) => p.dynamic(),
            Err(_) => return Ok(false),
        },
    };

    let Ok(identities) = agent.request_identities().await else {
        return Ok(false);
    };

    for id in identities {
        if let russh::keys::agent::AgentIdentity::PublicKey { key, .. } = id {
            let hash_alg = if matches!(key.algorithm(), russh::keys::Algorithm::Rsa { .. }) {
                handle
                    .best_supported_rsa_hash()
                    .await
                    .ok()
                    .flatten()
                    .flatten()
            } else {
                None
            };
            if let Ok(res) = handle
                .authenticate_publickey_with(user, key, hash_alg, &mut agent)
                .await
            {
                if res.success() {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}
