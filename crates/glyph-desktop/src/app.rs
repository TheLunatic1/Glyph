use crate::theme::{
    dark_900_bg, glyph_theme, APP_VERSION, FONT_UI, IBM_PLEX_MONO_BOLD, IBM_PLEX_MONO_MEDIUM,
    IBM_PLEX_MONO_REGULAR, IBM_PLEX_SANS_BOLD, IBM_PLEX_SANS_MEDIUM, IBM_PLEX_SANS_REGULAR,
    IBM_PLEX_SANS_SEMIBOLD, LOGO_PNG,
};
use crate::tray::{TrayCommand, TrayManager};
use crate::ui::commands_page::{self, CommandsPageMsg, CommandsPageState};
use crate::ui::containers_page::{
    self, ContainerSubTab, ContainerTabDetail, ContainersPageMsg, ContainersPageState,
};
use crate::ui::dashboard::{self, DashboardMsg, DashboardState};
use crate::ui::icons;
use crate::ui::manager::{self, ManagerMsg, ServerFormState};
use crate::ui::modals::{
    self, ConnectingModalState, DeleteConfirmState, ExportModalState, HostKeyPromptUi,
    ImportModalState, ImportStep, ManagerModal, MobileModalState, ModalMsg, SshKeysModalState,
};
use crate::ui::os_logo;
use crate::ui::quick_terminal::{QuickProtocol, QuickTransport};
use crate::ui::secrets_page::{self, SecretsBundleMode, SecretsPageMsg, SecretsPageState};
use crate::ui::sftp_page::{self, SftpEditorState, SftpPageMsg, SftpPageState, SftpPrompt};
use crate::ui::sidebar::{self, ServerTab, SidebarMsg};
use crate::ui::splash;
use crate::ui::terminal_page::{self, TerminalPageMsg, TerminalPageState};
use crate::ui::titlebar::{self, TitleBarMsg};
use crate::ui::tunnels_page::{self, TunnelsPageMsg, TunnelsPageState, UiTunnelDirection};
use crate::ui::update_modal::{self, AppUpdateState, UpdateMsg, UpdateStage};
use crate::ui::vnc_page::{self, VncPageMsg, VncPageState};
use glyph_api::{
    default_mcp_token_path, ensure_mcp_token, start_local_api, ApiHostKeyPolicy, ApiUiEvent,
    ApiUiTab, LocalApiHandle, LocalVaultApiController, DEFAULT_API_PORT,
};
use glyph_core::containers::{ContainerDetails, ContainerSummary};
use glyph_core::mcp_config::{
    auto_install_mcp_client, build_config_snippet, scan_mcp_clients, write_mcp_config_at,
};
use glyph_core::models::{
    AuthMethod, CommandSnippet, KnownHostEntry, SecretSummary, ServerEntry, ServerSummary,
    SshKeySummary,
};
use glyph_core::sftp::{join_remote_path, SftpEntry, SftpProgress};
use glyph_core::ssh::{
    HostKeyPrompt, SessionEvent, ShellEvent, ShellHandle, SshSession, TrustedHostRecord,
};
use glyph_core::tunnels::TunnelInfo;
use glyph_core::updater::{
    download_update_asset, ensure_rustls_ring_provider, fetch_latest_release, is_newer_version,
    launch_update_installer, DownloadedUpdate, ReleaseInfo, UpdateDownloadProgress,
    UpdateInstallKind,
};
use glyph_core::vault::VaultManager;
use glyph_terminal::TerminalEvent;
use glyph_vnc::{connect_vnc_stream, VncClientHandle, VncEvent, VncUiEvent};
use iced::futures::future::pending;
use iced::futures::{SinkExt, Stream};
use iced::widget::{column, container, row, stack, text_editor};
use iced::{window, Element, Length, Size, Subscription, Task, Theme};
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot, Mutex as AsyncMutex};

// ── Global Async Event Bus & TOFU Responder Registry ──────────────────────────

#[derive(Clone)]
#[allow(clippy::large_enum_variant)]
pub enum BusEvent {
    Session(window::Id, SessionEvent),
    ShellOpened {
        window_id: window::Id,
        tab_id: String,
        handle: ShellHandle,
    },
    ConnectLog {
        server_id: String,
        line: String,
    },
    HostKeyPrompt {
        server_id: String,
        prompt_id: u64,
        host: String,
        port: u16,
        algorithm: String,
        fingerprint_sha256: String,
        mismatch_expected: Option<String>,
    },
    ConnectSucceeded {
        server_id: String,
        session: SshSession,
        detected_os: Option<String>,
        newly_trusted: Vec<TrustedHostRecord>,
        event_tx: mpsc::UnboundedSender<SessionEvent>,
        window_slot: Arc<Mutex<Option<window::Id>>>,
    },
    ConnectFailed {
        server_id: String,
        error: String,
    },
    SshKeyDeployed {
        key_id: String,
        server_id: String,
        result: Result<String, String>,
    },
    Api(ApiUiEvent),
    SftpListed(window::Id, String, Result<Vec<SftpEntry>, String>),
    SftpFileRead(window::Id, String, Result<String, String>),
    SftpFileSaved(window::Id, String, Result<(), String>),
    SftpActionDone(window::Id, Result<String, String>),
    SftpProgress(window::Id, SftpProgress),
    SftpPeerListed(window::Id, String, Result<Vec<SftpEntry>, String>),
    SftpTransferFinished(window::Id, String, Result<String, String>),
    QuickTerminalData(window::Id, Vec<u8>),
    QuickTerminalClosed(window::Id, Option<String>),
    QuickTelnetConnected(window::Id, Result<glyph_core::telnet::TelnetHandle, String>),
    GroupRunHostDone {
        server_id: String,
        result: Result<String, String>,
    },
    TunnelsListed(window::Id, Vec<TunnelInfo>),
    TunnelOpDone(window::Id, Result<Vec<TunnelInfo>, String>),
    ContainersListed(window::Id, Result<Vec<ContainerSummary>, String>),
    ContainerActionDone(window::Id, Result<(), String>),
    ContainerDetailLoaded {
        window_id: window::Id,
        id: String,
        logs: Result<String, String>,
        inspect: Result<ContainerDetails, String>,
    },
    ContainerUpdateDone(window::Id, String, Result<String, String>),
    ComposeDone(window::Id, Result<String, String>),
    VncConnected(window::Id, Result<VncClientHandle, String>),
    VncEvent(window::Id, VncEvent),
    OsIconFetched(String, Vec<u8>),
    UpdateDownloadProgress(UpdateDownloadProgress),
    UpdateDownloadFinished(Result<DownloadedUpdate, String>),
}

impl fmt::Debug for BusEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Session(wid, ev) => f.debug_tuple("Session").field(wid).field(ev).finish(),
            Self::ShellOpened {
                window_id, tab_id, ..
            } => f
                .debug_struct("ShellOpened")
                .field("window_id", window_id)
                .field("tab_id", tab_id)
                .finish(),
            Self::ConnectLog { server_id, line } => f
                .debug_struct("ConnectLog")
                .field("server_id", server_id)
                .field("line", line)
                .finish(),
            Self::HostKeyPrompt {
                server_id,
                prompt_id,
                host,
                port,
                ..
            } => f
                .debug_struct("HostKeyPrompt")
                .field("server_id", server_id)
                .field("prompt_id", prompt_id)
                .field("host", host)
                .field("port", port)
                .finish(),
            Self::ConnectSucceeded {
                server_id,
                detected_os,
                ..
            } => f
                .debug_struct("ConnectSucceeded")
                .field("server_id", server_id)
                .field("detected_os", detected_os)
                .finish(),
            Self::ConnectFailed { server_id, error } => f
                .debug_struct("ConnectFailed")
                .field("server_id", server_id)
                .field("error", error)
                .finish(),
            Self::SshKeyDeployed {
                key_id,
                server_id,
                result,
            } => f
                .debug_struct("SshKeyDeployed")
                .field("key_id", key_id)
                .field("server_id", server_id)
                .field("result", result)
                .finish(),
            Self::Api(ev) => f.debug_tuple("Api").field(ev).finish(),
            Self::SftpListed(wid, path, _) => {
                f.debug_tuple("SftpListed").field(wid).field(path).finish()
            }
            Self::SftpFileRead(wid, path, _) => {
                f.debug_tuple("SftpFileRead").field(wid).field(path).finish()
            }
            Self::SftpFileSaved(wid, path, _) => {
                f.debug_tuple("SftpFileSaved").field(wid).field(path).finish()
            }
            Self::SftpActionDone(wid, _) => f.debug_tuple("SftpActionDone").field(wid).finish(),
            Self::SftpProgress(wid, p) => {
                f.debug_tuple("SftpProgress").field(wid).field(&p.filename).finish()
            }
            Self::SftpPeerListed(wid, path, _) => {
                f.debug_tuple("SftpPeerListed").field(wid).field(path).finish()
            }
            Self::SftpTransferFinished(wid, tid, _) => {
                f.debug_tuple("SftpTransferFinished").field(wid).field(tid).finish()
            }
            Self::QuickTerminalData(wid, b) => {
                f.debug_tuple("QuickTerminalData").field(wid).field(&b.len()).finish()
            }
            Self::QuickTerminalClosed(wid, _) => {
                f.debug_tuple("QuickTerminalClosed").field(wid).finish()
            }
            Self::QuickTelnetConnected(wid, _) => {
                f.debug_tuple("QuickTelnetConnected").field(wid).finish()
            }
            Self::GroupRunHostDone { server_id, .. } => f
                .debug_struct("GroupRunHostDone")
                .field("server_id", server_id)
                .finish(),
            Self::TunnelsListed(wid, list) => {
                f.debug_tuple("TunnelsListed").field(wid).field(&list.len()).finish()
            }
            Self::TunnelOpDone(wid, _) => f.debug_tuple("TunnelOpDone").field(wid).finish(),
            Self::ContainersListed(wid, _) => f.debug_tuple("ContainersListed").field(wid).finish(),
            Self::ContainerActionDone(wid, _) => {
                f.debug_tuple("ContainerActionDone").field(wid).finish()
            }
            Self::ContainerDetailLoaded { window_id, id, .. } => f
                .debug_struct("ContainerDetailLoaded")
                .field("window_id", window_id)
                .field("id", id)
                .finish(),
            Self::ContainerUpdateDone(wid, id, _) => f
                .debug_tuple("ContainerUpdateDone")
                .field(wid)
                .field(id)
                .finish(),
            Self::ComposeDone(wid, _) => f.debug_tuple("ComposeDone").field(wid).finish(),
            Self::VncConnected(wid, _) => f.debug_tuple("VncConnected").field(wid).finish(),
            Self::VncEvent(wid, _) => f.debug_tuple("VncEvent").field(wid).finish(),
            Self::OsIconFetched(slug, bytes) => f
                .debug_tuple("OsIconFetched")
                .field(slug)
                .field(&bytes.len())
                .finish(),
            Self::UpdateDownloadProgress(p) => f
                .debug_struct("UpdateDownloadProgress")
                .field("percent", &p.percent)
                .finish(),
            Self::UpdateDownloadFinished(r) => f
                .debug_tuple("UpdateDownloadFinished")
                .field(&r.as_ref().map(|d| &d.asset_name))
                .finish(),
        }
    }
}

type BusChannelPair = (
    mpsc::UnboundedSender<BusEvent>,
    Mutex<Option<mpsc::UnboundedReceiver<BusEvent>>>,
);

static BUS: OnceLock<BusChannelPair> = OnceLock::new();

fn bus_cell() -> &'static BusChannelPair {
    BUS.get_or_init(|| {
        let (tx, rx) = mpsc::unbounded_channel();
        (tx, Mutex::new(Some(rx)))
    })
}

pub fn app_bus_sender() -> mpsc::UnboundedSender<BusEvent> {
    bus_cell().0.clone()
}

static HOST_KEY_RESPONDERS: OnceLock<Mutex<HashMap<u64, oneshot::Sender<bool>>>> = OnceLock::new();
static PROMPT_SEQ: AtomicU64 = AtomicU64::new(1);

fn responders_map() -> &'static Mutex<HashMap<u64, oneshot::Sender<bool>>> {
    HOST_KEY_RESPONDERS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn bus_stream() -> impl Stream<Item = BusEvent> {
    iced::stream::channel(256, async |mut output| {
        let rx_opt = bus_cell().1.lock().ok().and_then(|mut g| g.take());
        if let Some(mut rx) = rx_opt {
            while let Some(ev) = rx.recv().await {
                if output.send(ev).await.is_err() {
                    break;
                }
            }
        }
        pending::<()>().await;
    })
}

static API_HANDLE: OnceLock<Mutex<Option<LocalApiHandle>>> = OnceLock::new();

fn api_handle_cell() -> &'static Mutex<Option<LocalApiHandle>> {
    API_HANDLE.get_or_init(|| Mutex::new(None))
}

pub(crate) fn effective_api_port() -> u16 {
    std::env::var("GLYPH_API_PORT")
        .ok()
        .and_then(|s| s.trim().parse::<u16>().ok())
        .filter(|&p| p > 0)
        .unwrap_or(DEFAULT_API_PORT)
}

pub(crate) fn effective_mcp_token_path() -> anyhow::Result<PathBuf> {
    if let Ok(raw) = std::env::var("GLYPH_MCP_TOKEN_FILE") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }
    default_mcp_token_path()
}

fn start_local_api_task(
    vault: Arc<Mutex<VaultManager>>,
    sessions: Arc<AsyncMutex<HashMap<String, SshSession>>>,
) -> Task<Message> {
    Task::perform(
        async move {
            let token_path = effective_mcp_token_path().map_err(|e| e.to_string())?;
            let token = ensure_mcp_token(&token_path).map_err(|e| e.to_string())?;
            let (ui_tx, mut ui_rx) = mpsc::unbounded_channel::<ApiUiEvent>();
            let bus = app_bus_sender();
            tokio::spawn(async move {
                while let Some(ev) = ui_rx.recv().await {
                    let _ = bus.send(BusEvent::Api(ev));
                }
            });

            let controller = Arc::new(LocalVaultApiController::new(
                vault,
                sessions,
                Some(ui_tx),
                ApiHostKeyPolicy::AutoTrustFirstUse,
            ));
            let addr = SocketAddr::from(([127, 0, 0, 1], effective_api_port()));
            let handle = start_local_api(addr, token, controller)
                .await
                .map_err(|e| e.to_string())?;
            let bound_port = handle.addr().port();
            if let Ok(mut guard) = api_handle_cell().lock() {
                *guard = Some(handle);
            }
            Ok(bound_port)
        },
        Message::LocalApiStarted,
    )
}

fn startup_update_check_task() -> Task<Message> {
    Task::perform(
        async {
            tokio::time::sleep(Duration::from_secs(4)).await;
            fetch_latest_release("TheLunatic1/Glyph", None)
                .await
                .map_err(|e| e.to_string())
        },
        |result| Message::DesktopUpdateChecked {
            silent: true,
            result,
        },
    )
}

// ── Top-Level Messages & State ────────────────────────────────────────────────

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum Message {
    Tick,
    LocalApiStarted(Result<u16, String>),
    Bus(BusEvent),
    WindowCloseRequested(window::Id),
    WindowOpened(window::Id),
    WindowResized(window::Id, Size),
    TitleBar(window::Id, TitleBarMsg),
    Manager(ManagerMsg),
    Modal(ModalMsg),
    Sidebar(window::Id, SidebarMsg),
    Dashboard(window::Id, DashboardMsg),
    Terminal(window::Id, TerminalPageMsg),
    Sftp(window::Id, SftpPageMsg),
    Commands(window::Id, CommandsPageMsg),
    Secrets(window::Id, SecretsPageMsg),
    Tunnels(window::Id, TunnelsPageMsg),
    Containers(window::Id, ContainersPageMsg),
    Vnc(window::Id, VncPageMsg),

    FileHovered(window::Id),
    FileDropped(window::Id, PathBuf),
    FilesHoveredLeft(window::Id),

    UserActivity,
    LockScreen(crate::ui::lock_screen::LockScreenMsg),
    TogglePalette,
    GlobalEscape,
    PaletteKeyNav(bool),
    Palette(crate::ui::palette::PaletteMsg),
    TermiusModalAction(crate::ui::termius_modals::TermiusModalMsg),
    QuickTerminal(window::Id, crate::ui::quick_terminal::QuickTerminalMsg),
    IdentityKeyFilePicked(Option<String>),
    ImportHostsSshFilePicked(Option<String>),
    ExportSshConfigDone(Result<Option<String>, String>),
    ImportPuttyFileLoaded(Result<Option<(String, String)>, String>),
    CloudHostsFetched(Result<Vec<glyph_core::models::ServerConfigInput>, String>),
    SyncFolderPicked(Option<String>),
    SyncFinished(Result<String, String>),

    FormPrivateKeyPicked(Option<String>),
    ExportFinished(Result<Option<String>, String>),
    ImportFileLoaded(Result<Option<(String, String)>, String>),
    SshKeyFileLoaded(Result<Option<(String, String)>, String>),
    CustomMcpPathPicked(Option<String>),
    MobileReleaseFetched(Result<ReleaseInfo, String>),
    DesktopUpdateChecked {
        silent: bool,
        result: Result<ReleaseInfo, String>,
    },
    Update(UpdateMsg),
    SftpUploadPicked(window::Id, Vec<PathBuf>),
    SftpDownloadSavePicked(window::Id, String, Option<PathBuf>),
    SecretsExportDone(window::Id, Result<Option<String>, String>),
    SecretsImportLoaded(window::Id, Result<Option<String>, String>),
}

pub(crate) struct ServerWindowState {
    pub(crate) server: ServerSummary,
    pub(crate) session: SshSession,
    pub(crate) active_tab: ServerTab,
    pub(crate) dashboard: DashboardState,
    pub(crate) terminal: TerminalPageState,
    pub(crate) sftp: SftpPageState,
    pub(crate) commands: CommandsPageState,
    pub(crate) secrets: SecretsPageState,
    pub(crate) tunnels: TunnelsPageState,
    pub(crate) containers: ContainersPageState,
    pub(crate) vnc: VncPageState,
    pub(crate) last_container_poll: Instant,
    pub(crate) cached_commands: Vec<CommandSnippet>,
    pub(crate) cached_secrets: Vec<SecretSummary>,
    pub(crate) window_size: Size,
    pub(crate) closing: bool,
}

impl ServerWindowState {
    fn new(
        server: ServerSummary,
        session: SshSession,
        cached_commands: Vec<CommandSnippet>,
        cached_secrets: Vec<SecretSummary>,
    ) -> Self {
        Self {
            server,
            session,
            active_tab: ServerTab::Dashboard,
            dashboard: DashboardState::default(),
            terminal: TerminalPageState::default(),
            sftp: SftpPageState::default(),
            commands: CommandsPageState::default(),
            secrets: SecretsPageState::default(),
            tunnels: TunnelsPageState::default(),
            containers: ContainersPageState::default(),
            vnc: VncPageState::default(),
            last_container_poll: Instant::now(),
            cached_commands,
            cached_secrets,
            window_size: Size::new(1280.0, 820.0),
            closing: false,
        }
    }
}

pub struct GlyphApp {
    pub(crate) vault: Arc<Mutex<VaultManager>>,
    pub(crate) api_sessions: Arc<AsyncMutex<HashMap<String, SshSession>>>,
    pub(crate) servers: Vec<ServerSummary>,
    pub(crate) ssh_keys: Vec<SshKeySummary>,
    pub(crate) known_hosts: Vec<KnownHostEntry>,
    pub(crate) tray: Option<TrayManager>,

    pub(crate) groups: Vec<glyph_core::models::HostGroup>,
    pub(crate) identities: Vec<glyph_core::models::IdentitySummary>,
    pub(crate) snippet_packages: Vec<glyph_core::models::SnippetPackage>,
    pub(crate) highlight_rules: Vec<glyph_core::models::HighlightRule>,
    pub(crate) compiled_highlights: Vec<glyph_terminal::CompiledHighlightRule>,
    pub(crate) app_lock_cfg: glyph_core::models::AppLockConfig,

    pub(crate) manager_window: Option<window::Id>,
    pub(crate) manager_size: Size,
    pub(crate) manager_visible: bool,
    pub(crate) manager_search: String,
    pub(crate) selected_group_filter: Option<String>,
    pub(crate) selected_tag_filter: Option<String>,
    pub(crate) show_splash: bool,
    pub(crate) splash_started_at: Option<Instant>,
    pub(crate) splash_alpha: f32,
    pub(crate) boot_tick_count: u8,
    pub(crate) disconnect_banner: Option<String>,
    pub(crate) form: Option<ServerFormState>,
    pub(crate) modal: Option<ManagerModal>,
    pub(crate) termius_modal: Option<crate::ui::termius_modals::TermiusModal>,
    pub(crate) termius_modal_window: Option<window::Id>,
    pub(crate) palette: crate::ui::palette::CommandPaletteState,
    pub(crate) locked: bool,
    pub(crate) lock_screen: crate::ui::lock_screen::LockScreenState,
    pub(crate) last_activity: Instant,
    pub(crate) connect_started_at: Option<Instant>,

    pub(crate) server_windows: BTreeMap<window::Id, ServerWindowState>,
    pub(crate) quick_windows: BTreeMap<window::Id, crate::ui::quick_terminal::QuickTerminalState>,
    pub(crate) transfer_cancels: HashMap<String, tokio_util::sync::CancellationToken>,
    pub(crate) updater: AppUpdateState,
    pub(crate) update_cancel: Option<tokio_util::sync::CancellationToken>,
    pub(crate) is_quitting: bool,
}

pub fn run() -> iced::Result {
    iced::daemon(GlyphApp::boot, GlyphApp::update, GlyphApp::view)
        .title(GlyphApp::title)
        .theme(GlyphApp::theme)
        .subscription(GlyphApp::subscription)
        .font(IBM_PLEX_SANS_REGULAR)
        .font(IBM_PLEX_SANS_MEDIUM)
        .font(IBM_PLEX_SANS_SEMIBOLD)
        .font(IBM_PLEX_SANS_BOLD)
        .font(IBM_PLEX_MONO_REGULAR)
        .font(IBM_PLEX_MONO_MEDIUM)
        .font(IBM_PLEX_MONO_BOLD)
        .font(icons::LUCIDE_FONT_BYTES)
        .default_font(FONT_UI)
        .antialiasing(true)
        .run()
}

impl GlyphApp {
    fn boot() -> (Self, Task<Message>) {
        ensure_rustls_ring_provider();
        let vault_raw =
            VaultManager::open_default().expect("Failed to initialize Glyph encrypted Vault");
        os_logo::set_icon_cache_dir(vault_raw.data_dir().join("os-icons"));
        let servers = vault_raw.list_servers();
        let ssh_keys = vault_raw.list_ssh_keys();
        let known_hosts = vault_raw.list_known_hosts().to_vec();
        let vault = Arc::new(Mutex::new(vault_raw));
        let api_sessions = Arc::new(AsyncMutex::new(HashMap::new()));
        let tray = TrayManager::init(&servers);

        let (manager_id, open_task) = window::open(manager_window_settings());
        let api_task = start_local_api_task(Arc::clone(&vault), Arc::clone(&api_sessions));

        let mut app = Self {
            vault,
            api_sessions,
            servers,
            ssh_keys,
            known_hosts,
            tray,
            groups: Vec::new(),
            identities: Vec::new(),
            snippet_packages: Vec::new(),
            highlight_rules: Vec::new(),
            compiled_highlights: Vec::new(),
            app_lock_cfg: glyph_core::models::AppLockConfig::default(),
            manager_window: Some(manager_id),
            manager_size: Size::new(1200.0, 800.0),
            manager_visible: true,
            manager_search: String::new(),
            selected_group_filter: None,
            selected_tag_filter: None,
            show_splash: true,
            splash_started_at: None,
            splash_alpha: 1.0,
            boot_tick_count: 0,
            disconnect_banner: None,
            form: None,
            modal: None,
            termius_modal: None,
            termius_modal_window: None,
            palette: crate::ui::palette::CommandPaletteState::default(),
            locked: false,
            lock_screen: crate::ui::lock_screen::LockScreenState::default(),
            last_activity: Instant::now(),
            connect_started_at: None,
            server_windows: BTreeMap::new(),
            quick_windows: BTreeMap::new(),
            transfer_cancels: HashMap::new(),
            updater: AppUpdateState::default(),
            update_cancel: None,
            is_quitting: false,
        };
        app.reload_termius_caches();
        app.locked = app.app_lock_cfg.enabled;
        app.trigger_os_icon_fetches(app.servers.iter().map(|s| s.os.as_deref()).collect());

        let mut startup_tasks = vec![open_task.discard(), api_task];
        if std::env::var("GLYPH_DISABLE_AUTO_UPDATE").as_deref() != Ok("1") {
            startup_tasks.push(startup_update_check_task());
        }
        (app, Task::batch(startup_tasks))
    }

    pub(crate) fn with_vault<R>(&self, f: impl FnOnce(&mut VaultManager) -> R) -> R {
        let mut guard = self.vault.lock().expect("Vault mutex poisoned");
        f(&mut guard)
    }

    fn title(&self, id: window::Id) -> String {
        if let Some(srv) = self.server_windows.get(&id) {
            let label = display_server_name(&srv.server);
            format!(
                "Glyph — {label} ({}@{})",
                srv.server.username, srv.server.host
            )
        } else if let Some(qt) = self.quick_windows.get(&id) {
            qt.title()
        } else {
            "Glyph".to_string()
        }
    }

    fn theme(&self, _id: window::Id) -> Theme {
        glyph_theme()
    }

    fn subscription(&self) -> Subscription<Message> {
        let tick_ms = if self.show_splash { 50 } else { 120 };
        Subscription::batch([
            Subscription::run(bus_stream).map(Message::Bus),
            window::close_requests().map(Message::WindowCloseRequested),
            iced::time::every(Duration::from_millis(tick_ms)).map(|_| Message::Tick),
            iced::event::listen_with(|ev, _status, wid| match ev {
                iced::Event::Window(window::Event::Opened { .. }) => {
                    Some(Message::WindowOpened(wid))
                }
                iced::Event::Window(window::Event::Resized(size)) => {
                    Some(Message::WindowResized(wid, size))
                }
                iced::Event::Window(window::Event::FileHovered(_)) => {
                    Some(Message::FileHovered(wid))
                }
                iced::Event::Window(window::Event::FileDropped(path)) => {
                    Some(Message::FileDropped(wid, path))
                }
                iced::Event::Window(window::Event::FilesHoveredLeft) => {
                    Some(Message::FilesHoveredLeft(wid))
                }
                iced::Event::Mouse(iced::mouse::Event::ButtonPressed(_)) => {
                    Some(Message::UserActivity)
                }
                iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                    key, modifiers, ..
                }) => {
                    if modifiers.command() {
                        if let iced::keyboard::Key::Character(c) = &key {
                            if c.eq_ignore_ascii_case("k") {
                                return Some(Message::TogglePalette);
                            }
                        }
                    }
                    match key {
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) => {
                            Some(Message::GlobalEscape)
                        }
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowUp) => {
                            Some(Message::PaletteKeyNav(false))
                        }
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowDown) => {
                            Some(Message::PaletteKeyNav(true))
                        }
                        _ => None,
                    }
                }
                _ => None,
            }),
        ])
    }

    pub(crate) fn trigger_os_icon_fetches(&self, os_list: Vec<Option<&str>>) {
        let mut slugs = Vec::new();
        for os in os_list.into_iter().flatten() {
            if let Some(slug) = os_logo::needs_fetch(os) {
                slugs.push(slug);
            }
        }
        for slug in slugs {
            tokio::spawn(async move {
                if let Some(bytes) = os_logo::fetch_devicon_svg(&slug).await {
                    let _ = app_bus_sender().send(BusEvent::OsIconFetched(slug, bytes));
                }
            });
        }
    }

    pub(crate) fn refresh_vault_cache(&mut self) {
        let (servers, ssh_keys, known_hosts) = self.with_vault(|v| {
            (
                v.list_servers(),
                v.list_ssh_keys(),
                v.list_known_hosts().to_vec(),
            )
        });
        self.servers = servers;
        self.ssh_keys = ssh_keys;
        self.known_hosts = known_hosts;
        self.trigger_os_icon_fetches(self.servers.iter().map(|s| s.os.as_deref()).collect());
        if let Some(tray) = &mut self.tray {
            tray.update_servers(&self.servers);
        }
        self.reload_termius_caches();
    }

    pub(crate) fn show_manager_window(&mut self) -> Task<Message> {
        if let Some(mid) = self.manager_window {
            self.manager_visible = true;
            Task::batch([
                window::set_mode(mid, window::Mode::Windowed),
                window::gain_focus(mid),
            ])
        } else {
            let (mid, task) = window::open(manager_window_settings());
            self.manager_window = Some(mid);
            self.manager_visible = true;
            task.discard()
        }
    }

    fn quit_all(&mut self) -> Task<Message> {
        self.is_quitting = true;
        let sessions: Vec<SshSession> = self
            .server_windows
            .values()
            .map(|s| s.session.clone())
            .collect();
        for srv in self.server_windows.values() {
            if let Some(vh) = &srv.vnc.handle {
                vh.disconnect();
            }
        }
        for qt in self.quick_windows.values() {
            qt.transport.close();
        }
        self.quick_windows.clear();
        self.server_windows.clear();
        let api_handle = api_handle_cell().lock().ok().and_then(|mut g| g.take());
        let api_sessions = Arc::clone(&self.api_sessions);
        tokio::spawn(async move {
            api_sessions.lock().await.clear();
            if let Some(h) = api_handle {
                h.shutdown().await;
            }
            for sess in sessions {
                sess.disconnect().await;
            }
        });
        iced::exit()
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        if !matches!(msg, Message::Tick | Message::Bus(_)) {
            self.last_activity = Instant::now();
        }
        match msg {
            Message::UserActivity => Task::none(),
            Message::LockScreen(lmsg) => self.handle_lock_screen_msg(lmsg),
            Message::TogglePalette => {
                if self.locked {
                    return Task::none();
                }
                self.palette.open = !self.palette.open;
                self.palette.query.clear();
                self.palette.selected_idx = 0;
                if self.palette.open {
                    self.rebuild_palette();
                }
                Task::none()
            }
            Message::GlobalEscape => {
                if self.palette.open {
                    self.palette.open = false;
                    self.palette.query.clear();
                } else if self.updater.modal_open {
                    if self.updater.stage != UpdateStage::Installing {
                        self.updater.modal_open = false;
                    }
                } else if self.termius_modal.is_some() {
                    self.termius_modal = None;
                    self.termius_modal_window = None;
                } else if self.modal.is_some() {
                    self.modal = None;
                }
                Task::none()
            }
            Message::PaletteKeyNav(down) => {
                self.handle_palette_nav(down);
                Task::none()
            }
            Message::Palette(pmsg) => self.handle_palette_msg(pmsg),
            Message::TermiusModalAction(tmsg) => self.handle_termius_modal_msg(tmsg),
            Message::QuickTerminal(wid, qmsg) => self.handle_quick_terminal_msg(wid, qmsg),
            Message::IdentityKeyFilePicked(opt) => {
                if let (Some(path), Some(crate::ui::termius_modals::TermiusModal::Identities(st))) =
                    (opt, &mut self.termius_modal)
                {
                    st.private_key_path = path;
                }
                Task::none()
            }
            Message::ImportHostsSshFilePicked(opt) => {
                if let (
                    Some(path),
                    Some(crate::ui::termius_modals::TermiusModal::ImportHosts(st)),
                ) = (opt, &mut self.termius_modal)
                {
                    st.ssh_config_path = path;
                }
                Task::none()
            }
            Message::ExportSshConfigDone(res) => {
                if let Some(crate::ui::termius_modals::TermiusModal::ImportHosts(st)) =
                    &mut self.termius_modal
                {
                    match res {
                        Ok(Some(p)) => {
                            st.error = None;
                            st.status = Some(format!("Exported OpenSSH config -> {p}"));
                        }
                        Ok(None) => {}
                        Err(e) => st.error = Some(e),
                    }
                }
                Task::none()
            }
            Message::ImportPuttyFileLoaded(res) => {
                if let Some(crate::ui::termius_modals::TermiusModal::ImportHosts(st)) =
                    &mut self.termius_modal
                {
                    match res {
                        Ok(Some((path, body))) => {
                            st.putty_csv_path = path;
                            st.putty_text = body.clone();
                            let parsed = if body.contains("\\Sessions\\")
                                || body.starts_with("Windows Registry Editor")
                            {
                                glyph_core::importers::parse_putty_reg(&body)
                            } else {
                                glyph_core::importers::parse_hosts_csv(&body)
                            };
                            st.selected = vec![true; parsed.len()];
                            st.status = Some(format!("Loaded {} host(s) from file.", parsed.len()));
                            st.error = None;
                            st.preview = parsed;
                        }
                        Ok(None) => {}
                        Err(e) => st.error = Some(e),
                    }
                }
                Task::none()
            }
            Message::CloudHostsFetched(res) => {
                if let Some(crate::ui::termius_modals::TermiusModal::ImportHosts(st)) =
                    &mut self.termius_modal
                {
                    st.loading = false;
                    match res {
                        Ok(list) => {
                            st.selected = vec![true; list.len()];
                            st.status =
                                Some(format!("Fetched {} cloud instance(s).", list.len()));
                            st.error = None;
                            st.preview = list;
                        }
                        Err(e) => st.error = Some(e),
                    }
                }
                Task::none()
            }
            Message::SyncFolderPicked(opt) => {
                if let (Some(path), Some(ManagerModal::Settings(st))) = (opt, &mut self.modal) {
                    st.sync_folder_path = path;
                }
                Task::none()
            }
            Message::SyncFinished(res) => {
                if let Some(ManagerModal::Settings(st)) = &mut self.modal {
                    st.sync_busy = false;
                    match res {
                        Ok(msg) => {
                            st.sync_error = None;
                            st.sync_status = Some(msg);
                        }
                        Err(e) => st.sync_error = Some(e),
                    }
                }
                self.refresh_vault_cache();
                Task::none()
            }
            Message::Tick => self.handle_tick(),
            Message::LocalApiStarted(res) => {
                if let Err(e) = res {
                    log::warn!("Glyph Local API failed to start: {e}");
                }
                Task::none()
            }
            Message::WindowCloseRequested(wid) => self.handle_window_close(wid),
            Message::WindowOpened(wid) => {
                if Some(wid) == self.manager_window && self.show_splash && self.splash_started_at.is_none() {
                    self.splash_started_at = Some(Instant::now());
                }
                Task::none()
            }
            Message::WindowResized(wid, size) => {
                if Some(wid) == self.manager_window {
                    self.manager_size = size;
                } else if let Some(srv) = self.server_windows.get_mut(&wid) {
                    srv.window_size = size;
                }
                Task::none()
            }
            Message::TitleBar(wid, tmsg) => {
                if matches!(tmsg, TitleBarMsg::Close) {
                    self.handle_window_close(wid)
                } else {
                    titlebar::handle_titlebar_action(wid, tmsg)
                }
            }
            Message::Bus(ev) => self.handle_bus_event(ev),
            Message::Manager(mmsg) => self.handle_manager_msg(mmsg),
            Message::Modal(mod_msg) => self.handle_modal_msg(mod_msg),
            Message::Sidebar(wid, smsg) => self.handle_sidebar_msg(wid, smsg),
            Message::Dashboard(wid, dmsg) => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    match dmsg {
                        DashboardMsg::OpenModal(m) => srv.dashboard.active_modal = Some(m),
                        DashboardMsg::CloseModal => srv.dashboard.active_modal = None,
                    }
                }
                Task::none()
            }
            Message::Terminal(wid, tmsg) => self.handle_terminal_msg(wid, tmsg),
            Message::Sftp(wid, smsg) => self.handle_sftp_msg(wid, smsg),
            Message::Commands(wid, cmsg) => self.handle_commands_msg(wid, cmsg),
            Message::Secrets(wid, smsg) => self.handle_secrets_msg(wid, smsg),
            Message::Tunnels(wid, tmsg) => self.handle_tunnels_msg(wid, tmsg),
            Message::Containers(wid, cmsg) => self.handle_containers_msg(wid, cmsg),
            Message::Vnc(wid, vmsg) => self.handle_vnc_msg(wid, vmsg),

            Message::FileHovered(wid) => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    if srv.active_tab == ServerTab::Sftp && srv.sftp.opened && srv.sftp.editor.is_none() {
                        srv.sftp.drag_hover = true;
                    }
                }
                Task::none()
            }
            Message::FilesHoveredLeft(wid) => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    srv.sftp.drag_hover = false;
                }
                Task::none()
            }
            Message::FileDropped(wid, path) => {
                let target_dir = self.server_windows.get_mut(&wid).and_then(|srv| {
                    srv.sftp.drag_hover = false;
                    (srv.active_tab == ServerTab::Sftp && srv.sftp.opened && path.is_file())
                        .then(|| srv.sftp.current_path.clone())
                });
                if let Some(dir) = target_dir {
                    self.enqueue_upload_files(wid, dir, vec![path]);
                }
                Task::none()
            }

            Message::FormPrivateKeyPicked(path_opt) => {
                if let (Some(form), Some(path)) = (&mut self.form, path_opt) {
                    form.private_key_path = path;
                }
                Task::none()
            }
            Message::ExportFinished(res) => {
                if let Some(ManagerModal::Export(st)) = &mut self.modal {
                    st.busy = false;
                    match res {
                        Ok(Some(path)) => {
                            st.error = None;
                            st.status = Some(format!("Backup exported securely to {path}"));
                        }
                        Ok(None) => {}
                        Err(e) => {
                            st.status = None;
                            st.error = Some(e);
                        }
                    }
                }
                Task::none()
            }
            Message::ImportFileLoaded(res) => {
                if let Some(ManagerModal::Import(st)) = &mut self.modal {
                    match res {
                        Ok(Some((path, content))) => {
                            st.file_path = Some(path);
                            st.encrypted_content = Some(content);
                            st.error = None;
                        }
                        Ok(None) => {}
                        Err(e) => st.error = Some(e),
                    }
                }
                Task::none()
            }
            Message::SshKeyFileLoaded(res) => {
                if let Some(ManagerModal::SshKeys(st)) = &mut self.modal {
                    match res {
                        Ok(Some((path, pem))) => {
                            if st.imp_name.trim().is_empty() {
                                if let Some(stem) = Path::new(&path)
                                    .file_name()
                                    .and_then(|s| s.to_str())
                                {
                                    st.imp_name = stem.to_string();
                                }
                            }
                            st.imp_path = Some(path);
                            st.imp_pem = Some(pem);
                            st.error = None;
                        }
                        Ok(None) => {}
                        Err(e) => st.error = Some(e),
                    }
                }
                Task::none()
            }
            Message::CustomMcpPathPicked(path_opt) => {
                if let (Some(ManagerModal::Settings(st)), Some(p)) = (&mut self.modal, path_opt) {
                    st.custom_path = p;
                }
                Task::none()
            }
            Message::MobileReleaseFetched(res) => {
                if let Some(ManagerModal::Mobile(st)) = &mut self.modal {
                    st.loading = false;
                    match res {
                        Ok(rel) => {
                            st.latest_version = Some(rel.tag_name);
                            st.direct_apk_url = rel.download_url;
                            st.page_url = rel.html_url;
                            st.error = None;
                            st.refresh_qr();
                        }
                        Err(e) => {
                            st.error = Some(format!("Offline / rate-limited ({e}); showing Releases page URL."));
                            st.refresh_qr();
                        }
                    }
                }
                Task::none()
            }
            Message::DesktopUpdateChecked { silent, result } => {
                if matches!(
                    self.updater.stage,
                    UpdateStage::Downloading | UpdateStage::Downloaded | UpdateStage::Installing
                ) {
                    return Task::none();
                }
                match result {
                    Ok(rel) => {
                        let newer = is_newer_version(APP_VERSION, &rel.version);
                        self.updater.info = Some(rel);
                        self.updater.error = None;
                        if newer {
                            self.updater.stage = UpdateStage::Available;
                            self.updater.banner_dismissed = false;
                        } else {
                            self.updater.stage = if silent {
                                UpdateStage::Idle
                            } else {
                                UpdateStage::UpToDate
                            };
                        }
                    }
                    Err(e) => {
                        if silent {
                            log::warn!("Silent update check failed: {e}");
                            if self.updater.stage == UpdateStage::Checking {
                                self.updater.stage = UpdateStage::Idle;
                            }
                        } else {
                            self.updater.stage = UpdateStage::Error;
                            self.updater.error = Some(e);
                        }
                    }
                }
                Task::none()
            }
            Message::Update(umsg) => self.handle_update_msg(umsg),
            Message::SftpUploadPicked(wid, paths) => {
                if !paths.is_empty() {
                    if let Some(dir) = self.server_windows.get(&wid).map(|s| s.sftp.current_path.clone()) {
                        self.enqueue_upload_files(wid, dir, paths);
                    }
                }
                Task::none()
            }
            Message::SftpDownloadSavePicked(wid, remote_path, save_opt) => {
                if let Some(local_path) = save_opt {
                    self.enqueue_download_file(wid, remote_path, local_path);
                }
                Task::none()
            }
            Message::SecretsExportDone(wid, res) => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    srv.secrets.bundle_modal = None;
                    srv.secrets.bundle_password.clear();
                    match res {
                        Ok(Some(path)) => {
                            srv.secrets.error = None;
                            srv.secrets.status = Some(format!("Secrets exported to {path}"));
                        }
                        Ok(None) => {}
                        Err(e) => srv.secrets.error = Some(e),
                    }
                }
                Task::none()
            }
            Message::SecretsImportLoaded(wid, res) => {
                let Some(srv) = self.server_windows.get_mut(&wid) else {
                    return Task::none();
                };
                match res {
                    Ok(Some(content)) => {
                        let pw = srv.secrets.bundle_password.clone();
                        let sid = srv.server.id.clone();
                        let import_res = self.vault.lock().map_err(|_| anyhow::anyhow!("Vault busy")).and_then(|mut v| {
                            let n = v.import_secrets_encrypted(&sid, &content, &pw)?;
                            Ok((n, v.list_secrets(&sid)))
                        });
                        srv.secrets.bundle_modal = None;
                        srv.secrets.bundle_password.clear();
                        match import_res {
                            Ok((count, list)) => {
                                srv.cached_secrets = list;
                                srv.secrets.error = None;
                                srv.secrets.status = Some(format!("Imported {count} secrets into Vault."));
                            }
                            Err(e) => srv.secrets.error = Some(e.to_string()),
                        }
                    }
                    Ok(None) => {}
                    Err(e) => srv.secrets.error = Some(e),
                }
                Task::none()
            }
        }
    }

    fn handle_tick(&mut self) -> Task<Message> {
        if self.show_splash {
            if self.splash_started_at.is_none() {
                self.boot_tick_count = self.boot_tick_count.saturating_add(1);
                if self.boot_tick_count >= 2 {
                    self.splash_started_at = Some(Instant::now());
                }
            }
            if let Some(started) = self.splash_started_at {
                let elapsed_ms = started.elapsed().as_millis() as u64;
                if elapsed_ms < 1150 {
                    self.splash_alpha = 1.0;
                } else if elapsed_ms < 1500 {
                    let t = (elapsed_ms - 1150) as f32 / 350.0;
                    self.splash_alpha = (1.0 - t).clamp(0.0, 1.0);
                } else {
                    self.splash_alpha = 0.0;
                    self.show_splash = false;
                }
            }
        }

        if !self.locked && self.app_lock_cfg.enabled && self.app_lock_cfg.idle_timeout_mins > 0 {
            let limit_secs = u64::from(self.app_lock_cfg.idle_timeout_mins) * 60;
            if self.last_activity.elapsed().as_secs() >= limit_secs {
                self.locked = true;
                self.lock_screen = crate::ui::lock_screen::LockScreenState {
                    idle_locked: true,
                    ..Default::default()
                };
            }
        }

        if let Some(ManagerModal::Connecting(st)) = &mut self.modal {
            if st.error.is_none() && st.host_key_prompt.is_none() {
                if let Some(started) = self.connect_started_at {
                    st.elapsed_ms = started.elapsed().as_millis() as u64;
                }
                st.blink_on = !st.blink_on;
            }
        }

        // Poll open container detail logs/stats every 3 seconds
        for (&wid, srv) in &mut self.server_windows {
            if srv.active_tab == ServerTab::Containers
                && srv.last_container_poll.elapsed() >= Duration::from_secs(3)
            {
                if let Some(cid) = srv.containers.active_tab.clone() {
                    if let Some(det) = srv.containers.details.get(&cid) {
                        if matches!(det.subtab, ContainerSubTab::Logs | ContainerSubTab::Inspect) {
                            srv.last_container_poll = Instant::now();
                            let running = det.summary.state.eq_ignore_ascii_case("running");
                            spawn_load_container_detail(wid, srv.session.clone(), cid, running);
                        }
                    }
                }
            }
        }

        let cmds = self
            .tray
            .as_ref()
            .map(TrayManager::drain_commands)
            .unwrap_or_default();

        let mut tasks = Vec::new();
        for cmd in cmds {
            match cmd {
                TrayCommand::ToggleMain => {
                    if let Some(mid) = self.manager_window {
                        if self.manager_visible {
                            self.manager_visible = false;
                            tasks.push(window::set_mode(mid, window::Mode::Hidden));
                        } else {
                            tasks.push(self.show_manager_window());
                        }
                    } else {
                        tasks.push(self.show_manager_window());
                    }
                }
                TrayCommand::OpenServer(server_id) => {
                    tasks.push(self.show_manager_window());
                    self.start_connect(server_id);
                }
                TrayCommand::Exit => {
                    return self.quit_all();
                }
            }
        }

        if tasks.is_empty() {
            Task::none()
        } else {
            Task::batch(tasks)
        }
    }

    pub(crate) fn handle_window_close(&mut self, wid: window::Id) -> Task<Message> {
        if Some(wid) == self.manager_window {
            if self.tray.is_some() && !self.is_quitting {
                self.manager_visible = false;
                return window::set_mode(wid, window::Mode::Hidden);
            }
            return self.quit_all();
        }

        if let Some(qt) = self.quick_windows.remove(&wid) {
            qt.transport.close();
            return window::close(wid);
        }

        if let Some(mut srv) = self.server_windows.remove(&wid) {
            srv.closing = true;
            if let Some(vh) = &srv.vnc.handle {
                vh.disconnect();
            }
            let sid = srv.server.id;
            let sess = srv.session;
            let api_sessions = Arc::clone(&self.api_sessions);
            tokio::spawn(async move {
                api_sessions.lock().await.remove(&sid);
                sess.disconnect().await;
            });
            self.rebuild_palette();
            return Task::batch([window::close(wid), self.show_manager_window()]);
        }

        window::close(wid)
    }

    fn handle_bus_event(&mut self, ev: BusEvent) -> Task<Message> {
        match ev {
            BusEvent::ConnectLog { server_id, line } => {
                if let Some(ManagerModal::Connecting(st)) = &mut self.modal {
                    if st.server_id == server_id {
                        st.logs.push(line);
                    }
                }
                Task::none()
            }

            BusEvent::HostKeyPrompt {
                server_id,
                prompt_id,
                host,
                port,
                algorithm,
                fingerprint_sha256,
                mismatch_expected,
            } => {
                if let Some(ManagerModal::Connecting(st)) = &mut self.modal {
                    if st.server_id == server_id {
                        st.host_key_prompt = Some(HostKeyPromptUi {
                            prompt_id,
                            host,
                            port,
                            algorithm,
                            fingerprint_sha256,
                            mismatch_expected,
                        });
                    }
                }
                Task::none()
            }

            BusEvent::ConnectSucceeded {
                server_id,
                session,
                detected_os,
                newly_trusted,
                event_tx,
                window_slot,
            } => {
                self.with_vault(|v| {
                    for rec in newly_trusted {
                        let _ = v.trust_host_key(
                            &rec.host,
                            rec.port,
                            &rec.algorithm,
                            &rec.fingerprint_sha256,
                        );
                    }
                    if detected_os.is_some() {
                        let _ = v.update_server_os(&server_id, detected_os.clone());
                    }
                });
                self.refresh_vault_cache();

                if matches!(&self.modal, Some(ManagerModal::Connecting(st)) if st.server_id == server_id)
                {
                    self.modal = None;
                }
                self.connect_started_at = None;

                let Some(server) = self.with_vault(|v| v.get_server(&server_id)) else {
                    tokio::spawn(async move {
                        session.disconnect().await;
                    });
                    return Task::none();
                };

                let (new_wid, open_task) = window::open(server_window_settings());
                if let Ok(mut guard) = window_slot.lock() {
                    *guard = Some(new_wid);
                }

                let (cached_commands, cached_secrets) =
                    self.with_vault(|v| (v.list_commands(&server.id), v.list_secrets(&server.id)));

                let api_sessions = Arc::clone(&self.api_sessions);
                let sid_api = server.id.clone();
                let sess_api = session.clone();
                tokio::spawn(async move {
                    api_sessions.lock().await.insert(sid_api, sess_api);
                });

                session.start_stat_polling(event_tx);
                terminal_page::spawn_open_shell(
                    session.clone(),
                    "tab-main".to_string(),
                    100,
                    30,
                    new_wid,
                );

                self.server_windows.insert(
                    new_wid,
                    ServerWindowState::new(server, session, cached_commands, cached_secrets),
                );
                self.init_server_termius_hooks(new_wid);
                self.rebuild_palette();

                Task::batch([open_task.discard(), window::gain_focus(new_wid)])
            }

            BusEvent::ConnectFailed { server_id, error } => {
                if let Some(ManagerModal::Connecting(st)) = &mut self.modal {
                    if st.server_id == server_id {
                        st.host_key_prompt = None;
                        st.error = Some(error);
                    }
                }
                Task::none()
            }

            BusEvent::ShellOpened {
                window_id,
                tab_id,
                handle,
            } => {
                let mut auto_cmd: Option<String> = None;
                if let Some(srv) = self.server_windows.get_mut(&window_id) {
                    srv.terminal.set_shell(&tab_id, handle);
                    if tab_id == "tab-main" && !srv.terminal.startup_sent {
                        srv.terminal.startup_sent = true;
                        let cmd = srv.server.startup_command.trim();
                        if !cmd.is_empty() {
                            auto_cmd = Some(cmd.to_string());
                        }
                    }
                }
                if let Some(cmd) = auto_cmd {
                    let mut payload = cmd.into_bytes();
                    payload.push(b'\r');
                    if let Some(srv) = self.server_windows.get(&window_id) {
                        srv.terminal.send_input(&tab_id, payload);
                    }
                }
                Task::none()
            }

            BusEvent::Session(wid, sev) => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    match sev {
                        SessionEvent::Status(_) => {}
                        SessionEvent::Stats(snap) => {
                            srv.dashboard.stats = snap;
                        }
                        SessionEvent::Shell(ShellEvent::Data { tab_id, bytes }) => {
                            if let Some(rec) = srv.terminal.recorders.get_mut(&tab_id) {
                                rec.record_output(&bytes);
                            }
                            srv.terminal.feed_output(&tab_id, &bytes);
                        }
                        SessionEvent::Shell(ShellEvent::Closed { tab_id }) => {
                            srv.terminal.mark_closed(&tab_id);
                        }
                        SessionEvent::Disconnected(reason) => {
                            if !srv.closing {
                                let name = display_server_name(&srv.server);
                                let sid = srv.server.id.clone();
                                if let Some(vh) = &srv.vnc.handle {
                                    vh.disconnect();
                                }
                                self.server_windows.remove(&wid);
                                let api_sessions = Arc::clone(&self.api_sessions);
                                tokio::spawn(async move {
                                    api_sessions.lock().await.remove(&sid);
                                });
                                self.disconnect_banner = Some(format!("{name}: {reason}"));
                                return Task::batch([
                                    window::close(wid),
                                    self.show_manager_window(),
                                ]);
                            }
                        }
                    }
                }
                Task::none()
            }

            BusEvent::SshKeyDeployed {
                key_id,
                server_id,
                result,
            } => {
                match result {
                    Ok(ok_msg) => {
                        self.with_vault(|v| {
                            let _ = v.set_server_vault_key(&server_id, &key_id);
                        });
                        self.refresh_vault_cache();
                        if let Some(ManagerModal::SshKeys(st)) = &mut self.modal {
                            st.busy = false;
                            st.deploy_key_id = None;
                            st.error = None;
                            st.status = Some(ok_msg);
                        }
                    }
                    Err(err_msg) => {
                        if let Some(ManagerModal::SshKeys(st)) = &mut self.modal {
                            st.busy = false;
                            st.status = None;
                            st.error = Some(err_msg);
                        }
                    }
                }
                Task::none()
            }

            BusEvent::Api(ev) => self.handle_api_ui_event(ev),

            BusEvent::SftpListed(wid, path, res) => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    srv.sftp.loading = false;
                    srv.sftp.opened = true;
                    match res {
                        Ok(entries) => {
                            srv.sftp.current_path = path;
                            srv.sftp.entries = entries;
                            srv.sftp.error = None;
                        }
                        Err(e) => srv.sftp.error = Some(e),
                    }
                }
                Task::none()
            }
            BusEvent::SftpFileRead(wid, path, res) => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    match res {
                        Ok(txt) => {
                            srv.sftp.error = None;
                            srv.sftp.editor = Some(SftpEditorState {
                                path,
                                content: text_editor::Content::with_text(&txt),
                                dirty: false,
                                saving: false,
                            });
                        }
                        Err(e) => srv.sftp.error = Some(e),
                    }
                }
                Task::none()
            }
            BusEvent::SftpFileSaved(wid, path, res) => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    if let Some(ed) = &mut srv.sftp.editor {
                        ed.saving = false;
                        if res.is_ok() {
                            ed.dirty = false;
                        }
                    }
                    match res {
                        Ok(()) => {
                            srv.sftp.error = None;
                            srv.sftp.status = Some(format!("Saved {path}"));
                            spawn_sftp_readdir(wid, srv.session.clone(), srv.sftp.current_path.clone());
                        }
                        Err(e) => srv.sftp.error = Some(e),
                    }
                }
                Task::none()
            }
            BusEvent::SftpActionDone(wid, res) => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    srv.sftp.transfer = None;
                    match res {
                        Ok(msg) => {
                            srv.sftp.error = None;
                            srv.sftp.status = Some(msg);
                            spawn_sftp_readdir(wid, srv.session.clone(), srv.sftp.current_path.clone());
                        }
                        Err(e) => srv.sftp.error = Some(e),
                    }
                }
                Task::none()
            }
            BusEvent::SftpProgress(wid, p) => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    if let Some(q) = srv
                        .sftp
                        .transfer_queue
                        .iter_mut()
                        .find(|q| q.id == p.transfer_id)
                    {
                        q.transferred = p.transferred;
                        q.total = p.total;
                    }
                    srv.sftp.transfer = Some(p);
                }
                Task::none()
            }
            BusEvent::SftpPeerListed(wid, path, res) => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    srv.sftp.remote_peer_loading = false;
                    match res {
                        Ok(entries) => {
                            srv.sftp.remote_peer_path = path;
                            srv.sftp.remote_peer_entries = entries;
                        }
                        Err(e) => srv.sftp.error = Some(e),
                    }
                }
                Task::none()
            }
            BusEvent::SftpTransferFinished(wid, tid, res) => {
                self.transfer_cancels.remove(&tid);
                let mut refresh_peer = None;
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    srv.sftp.transfer = None;
                    if let Some(q) = srv.sftp.transfer_queue.iter_mut().find(|q| q.id == tid) {
                        match &res {
                            Ok(_) => {
                                q.transferred = q.total;
                                q.status = crate::ui::sftp_page::TransferStatus::Done;
                            }
                            Err(e) if e.contains("cancelled") => {
                                q.status = crate::ui::sftp_page::TransferStatus::Cancelled;
                            }
                            Err(e) => {
                                q.status = crate::ui::sftp_page::TransferStatus::Failed(e.clone());
                            }
                        }
                    }
                    match res {
                        Ok(msg) => {
                            srv.sftp.error = None;
                            srv.sftp.status = Some(msg);
                            spawn_sftp_readdir(wid, srv.session.clone(), srv.sftp.current_path.clone());
                            if srv.sftp.dual_pane == crate::ui::sftp_page::DualPaneMode::LocalFs {
                                let lp = srv.sftp.local_path.clone();
                                if let Ok((p, list)) = crate::ui::sftp_page::list_local_dir(&lp) {
                                    srv.sftp.local_path = p;
                                    srv.sftp.local_entries = list;
                                }
                            } else if let crate::ui::sftp_page::DualPaneMode::RemoteServer(pwid) =
                                srv.sftp.dual_pane
                            {
                                refresh_peer = Some((pwid, srv.sftp.remote_peer_path.clone()));
                            }
                        }
                        Err(e) => srv.sftp.error = Some(e),
                    }
                }
                if let Some((pwid, peer_path)) = refresh_peer {
                    if let Some(psess) = self.server_windows.get(&pwid).map(|s| s.session.clone()) {
                        spawn_sftp_readdir_peer(wid, psess, peer_path);
                    }
                }
                Task::none()
            }
            BusEvent::QuickTerminalData(wid, bytes) => {
                if let Some(qt) = self.quick_windows.get_mut(&wid) {
                    let has_dsr = bytes.windows(4).any(|w| w == b"\x1b[6n");
                    qt.emulator.feed_bytes(&bytes);
                    if has_dsr && qt.protocol == QuickProtocol::LocalShell {
                        let (r, c) = qt.emulator.cursor_position_1based();
                        qt.transport.write(format!("\x1b[{r};{c}R").into_bytes());
                    }
                    if let Some(rec) = &mut qt.recorder {
                        rec.record_output(&bytes);
                    }
                }
                Task::none()
            }
            BusEvent::QuickTerminalClosed(wid, reason) => {
                if let Some(qt) = self.quick_windows.get_mut(&wid) {
                    qt.transport = QuickTransport::Disconnected;
                    if let Some(r) = reason {
                        qt.emulator.feed_bytes(format!("\r\n\x1b[33m[{r}]\x1b[0m\r\n").as_bytes());
                        qt.status = Some(r);
                    }
                }
                Task::none()
            }
            BusEvent::QuickTelnetConnected(wid, res) => {
                if let Some(qt) = self.quick_windows.get_mut(&wid) {
                    match res {
                        Ok(handle) => {
                            qt.transport = QuickTransport::Telnet(handle);
                            qt.error = None;
                            qt.status = Some("Telnet connected".to_string());
                        }
                        Err(e) => {
                            qt.transport = QuickTransport::Disconnected;
                            qt.error = Some(e);
                        }
                    }
                }
                Task::none()
            }
            BusEvent::GroupRunHostDone { server_id, result } => {
                if let Some(crate::ui::termius_modals::TermiusModal::GroupRun(st)) =
                    &mut self.termius_modal
                {
                    if let Some(row) = st.results.iter_mut().find(|r| r.server_id == server_id) {
                        match result {
                            Ok(out) => {
                                row.status = "Success".to_string();
                                row.output = if out.trim().is_empty() {
                                    "(no output)".to_string()
                                } else {
                                    out
                                };
                            }
                            Err(err) => {
                                row.status = "Failed".to_string();
                                row.output = err;
                            }
                        }
                    }
                    st.running = st.results.iter().any(|r| r.status == "Running");
                }
                Task::none()
            }

            BusEvent::TunnelsListed(wid, list) => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    srv.tunnels.busy = false;
                    srv.tunnels.tunnels = list;
                }
                Task::none()
            }
            BusEvent::TunnelOpDone(wid, res) => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    srv.tunnels.busy = false;
                    match res {
                        Ok(list) => {
                            srv.tunnels.tunnels = list;
                            srv.tunnels.bind_port.clear();
                            srv.tunnels.target_port.clear();
                            srv.tunnels.error = None;
                        }
                        Err(e) => srv.tunnels.error = Some(e),
                    }
                }
                Task::none()
            }

            BusEvent::ContainersListed(wid, res) => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    srv.containers.loading = false;
                    srv.containers.loaded_once = true;
                    match res {
                        Ok(list) => {
                            srv.containers.error = None;
                            for c in &list {
                                if let Some(det) = srv.containers.details.get_mut(&c.id) {
                                    det.summary = c.clone();
                                }
                            }
                            srv.containers.containers = list;
                        }
                        Err(e) => srv.containers.error = Some(e),
                    }
                }
                Task::none()
            }
            BusEvent::ContainerActionDone(wid, res) => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    match res {
                        Ok(()) => {
                            srv.containers.error = None;
                            spawn_list_containers(wid, srv.session.clone());
                        }
                        Err(e) => srv.containers.error = Some(e),
                    }
                }
                Task::none()
            }
            BusEvent::ContainerDetailLoaded {
                window_id,
                id,
                logs,
                inspect,
            } => {
                if let Some(srv) = self.server_windows.get_mut(&window_id) {
                    if let Some(det) = srv.containers.details.get_mut(&id) {
                        det.loading = false;
                        match logs {
                            Ok(txt) => det.logs = txt,
                            Err(e) => det.logs = format!("Failed to fetch logs: {e}"),
                        }
                        match inspect {
                            Ok(ins) => {
                                if (det.restart_policy.is_empty()
                                    || det.restart_policy == "unless-stopped")
                                    && !ins.restart_policy.is_empty()
                                    && ins.restart_policy != "—"
                                {
                                    det.restart_policy = ins.restart_policy.clone();
                                }
                                det.inspect = Some(ins);
                            }
                            Err(e) => srv.containers.error = Some(e),
                        }
                    }
                }
                Task::none()
            }
            BusEvent::ContainerUpdateDone(wid, id, res) => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    if let Some(det) = srv.containers.details.get_mut(&id) {
                        match res {
                            Ok(msg) => {
                                det.update_status = Some(if msg.trim().is_empty() {
                                    "Container settings updated.".to_string()
                                } else {
                                    msg
                                });
                            }
                            Err(e) => srv.containers.error = Some(e),
                        }
                    }
                }
                Task::none()
            }
            BusEvent::ComposeDone(wid, res) => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    srv.containers.compose_busy = false;
                    match res {
                        Ok(out) => {
                            srv.containers.compose_output = Some(out);
                            srv.containers.error = None;
                            spawn_list_containers(wid, srv.session.clone());
                        }
                        Err(e) => srv.containers.error = Some(e),
                    }
                }
                Task::none()
            }

            BusEvent::VncConnected(wid, res) => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    srv.vnc.connecting = false;
                    match res {
                        Ok(handle) => {
                            srv.vnc.handle = Some(handle);
                            srv.vnc.connected = true;
                            srv.vnc.error = None;
                        }
                        Err(e) => {
                            srv.vnc.connected = false;
                            srv.vnc.error = Some(e);
                        }
                    }
                }
                Task::none()
            }
            BusEvent::VncEvent(wid, vev) => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    match vev {
                        VncEvent::Connected {
                            desktop_name,
                            width,
                            height,
                        } => {
                            srv.vnc.desktop_name = desktop_name;
                            srv.vnc.width = width;
                            srv.vnc.height = height;
                            srv.vnc.connected = true;
                        }
                        VncEvent::Frame(frame) => {
                            srv.vnc.width = frame.width;
                            srv.vnc.height = frame.height;
                            srv.vnc.frame = Some(frame);
                        }
                        VncEvent::ServerClipboard(clip) => {
                            srv.vnc.server_clipboard = Some(clip);
                        }
                        VncEvent::Disconnected(reason) => {
                            srv.vnc.connected = false;
                            srv.vnc.connecting = false;
                            srv.vnc.handle = None;
                            if srv.vnc.error.is_none() {
                                srv.vnc.error = Some(reason);
                            }
                        }
                        VncEvent::Error(err) => {
                            srv.vnc.error = Some(err);
                        }
                    }
                }
                Task::none()
            }
            BusEvent::OsIconFetched(slug, bytes) => {
                os_logo::register_fetched_svg(&slug, bytes);
                Task::none()
            }
            BusEvent::UpdateDownloadProgress(prog) => {
                if self.updater.stage == UpdateStage::Downloading {
                    self.updater.progress = Some(prog);
                }
                Task::none()
            }
            BusEvent::UpdateDownloadFinished(res) => {
                self.update_cancel = None;
                match res {
                    Ok(dl) => {
                        self.updater.downloaded = Some(dl);
                        self.updater.stage = UpdateStage::Downloaded;
                        self.updater.modal_open = true;
                        self.updater.error = None;
                    }
                    Err(e) => {
                        let cancelled = e.to_ascii_lowercase().contains("cancel")
                            || self.updater.stage != UpdateStage::Downloading;
                        if cancelled {
                            self.updater.progress = None;
                            self.updater.stage = if self.updater.info.is_some() {
                                UpdateStage::Available
                            } else {
                                UpdateStage::Idle
                            };
                        } else {
                            self.updater.stage = UpdateStage::Error;
                            self.updater.error = Some(e);
                            self.updater.modal_open = true;
                        }
                    }
                }
                Task::none()
            }
        }
    }

    pub(crate) fn handle_manager_msg(&mut self, msg: ManagerMsg) -> Task<Message> {
        match msg {
            ManagerMsg::DismissDisconnectBanner => {
                self.disconnect_banner = None;
                Task::none()
            }
            ManagerMsg::SearchChanged(q) => {
                self.manager_search = q;
                Task::none()
            }
            ManagerMsg::OpenPalette => {
                self.palette.open = true;
                self.palette.query.clear();
                self.palette.selected_idx = 0;
                self.rebuild_palette();
                Task::none()
            }
            ManagerMsg::LockVaultNow => {
                self.locked = true;
                self.lock_screen = crate::ui::lock_screen::LockScreenState::default();
                Task::none()
            }
            ManagerMsg::OpenMobileModal => self.open_mobile_modal(),
            ManagerMsg::OpenSshKeysModal => {
                self.modal = Some(ManagerModal::SshKeys(SshKeysModalState::default()));
                Task::none()
            }
            ManagerMsg::OpenIdentitiesModal => {
                self.termius_modal_window = self.manager_window;
                self.termius_modal = Some(crate::ui::termius_modals::TermiusModal::Identities(
                    crate::ui::termius_modals::IdentitiesModalState::default(),
                ));
                Task::none()
            }
            ManagerMsg::OpenImportHostsModal => {
                self.termius_modal_window = self.manager_window;
                self.termius_modal = Some(crate::ui::termius_modals::TermiusModal::ImportHosts(
                    crate::ui::termius_modals::HostImportModalState::default(),
                ));
                Task::none()
            }
            ManagerMsg::SelectGroupFilter(gid) => {
                self.selected_group_filter = gid;
                Task::none()
            }
            ManagerMsg::SelectTagFilter(tag) => {
                self.selected_tag_filter = tag;
                Task::none()
            }
            ManagerMsg::OpenNewGroupModal => {
                self.termius_modal_window = self.manager_window;
                self.termius_modal = Some(crate::ui::termius_modals::TermiusModal::Group(
                    crate::ui::termius_modals::GroupModalState {
                        parent_id: self.selected_group_filter.clone(),
                        ..Default::default()
                    },
                ));
                Task::none()
            }
            ManagerMsg::OpenEditGroupModal(gid) => {
                if let Some(grp) = self.groups.iter().find(|g| g.id == gid).cloned() {
                    self.termius_modal_window = self.manager_window;
                    self.termius_modal = Some(crate::ui::termius_modals::TermiusModal::Group(
                        crate::ui::termius_modals::GroupModalState {
                            editing_id: Some(grp.id),
                            name: grp.name,
                            parent_id: grp.parent_id,
                            identity_id: grp.identity_id,
                            proxy_jump_id: grp.proxy_jump_id,
                            default_port: grp.default_port.unwrap_or(22).to_string(),
                            tags: grp.tags.join(", "),
                            error: None,
                        },
                    ));
                }
                Task::none()
            }
            ManagerMsg::OpenLocalTerminal => {
                self.open_quick_terminal(crate::ui::quick_terminal::QuickProtocol::LocalShell)
            }
            ManagerMsg::OpenSerialConsole => {
                self.open_quick_terminal(crate::ui::quick_terminal::QuickProtocol::Serial)
            }
            ManagerMsg::OpenTelnetConsole => {
                self.open_quick_terminal(crate::ui::quick_terminal::QuickProtocol::Telnet)
            }
            ManagerMsg::OpenImportModal => {
                self.modal = Some(ManagerModal::Import(ImportModalState::default()));
                Task::none()
            }
            ManagerMsg::OpenExportModal => {
                self.modal = Some(ManagerModal::Export(ExportModalState::new(&self.servers)));
                Task::none()
            }
            ManagerMsg::OpenSettingsModal => {
                self.open_settings_modal_tab(crate::ui::modals::SettingsTab::AiAgents);
                Task::none()
            }
            ManagerMsg::ToggleAddForm => {
                if matches!(&self.form, Some(f) if f.editing_id.is_none()) {
                    self.form = None;
                } else {
                    self.form = Some(ServerFormState {
                        group_id: self.selected_group_filter.clone(),
                        ..Default::default()
                    });
                }
                Task::none()
            }
            ManagerMsg::StartEditServer(id) => {
                if let Some(srv) = self.servers.iter().find(|s| s.id == id) {
                    self.form = Some(ServerFormState::from_summary(srv));
                }
                Task::none()
            }
            ManagerMsg::RequestDeleteServer(id) => {
                if let Some(srv) = self.servers.iter().find(|s| s.id == id) {
                    self.modal = Some(ManagerModal::DeleteConfirm(DeleteConfirmState {
                        server_id: srv.id.clone(),
                        server_name: display_server_name(srv),
                        host: srv.host.clone(),
                    }));
                }
                Task::none()
            }
            ManagerMsg::ConnectServer(id) => {
                self.disconnect_banner = None;
                self.start_connect(id);
                Task::none()
            }
            ManagerMsg::CancelForm => {
                self.form = None;
                Task::none()
            }
            ManagerMsg::FormNameChanged(v) => {
                if let Some(f) = &mut self.form {
                    f.name = v;
                }
                Task::none()
            }
            ManagerMsg::FormHostChanged(v) => {
                if let Some(f) = &mut self.form {
                    f.host = v;
                }
                Task::none()
            }
            ManagerMsg::FormPortChanged(v) => {
                if let Some(f) = &mut self.form {
                    f.port = v;
                }
                Task::none()
            }
            ManagerMsg::FormUsernameChanged(v) => {
                if let Some(f) = &mut self.form {
                    f.username = v;
                }
                Task::none()
            }
            ManagerMsg::FormPasswordChanged(v) => {
                if let Some(f) = &mut self.form {
                    f.password = v;
                }
                Task::none()
            }
            ManagerMsg::FormToggleShowPassword => {
                if let Some(f) = &mut self.form {
                    f.show_password = !f.show_password;
                }
                Task::none()
            }
            ManagerMsg::FormAuthMethodChanged(am) => {
                if let Some(f) = &mut self.form {
                    f.auth_method = am;
                    if am == AuthMethod::VaultKey || am == AuthMethod::KeyFile {
                        f.show_advanced = true;
                    }
                }
                Task::none()
            }
            ManagerMsg::FormToggleAdvanced => {
                if let Some(f) = &mut self.form {
                    f.show_advanced = !f.show_advanced;
                }
                Task::none()
            }
            ManagerMsg::FormSelectVaultKey(pick) => {
                if let Some(f) = &mut self.form {
                    f.ssh_key_id = pick.id.clone();
                    if f.ssh_key_id.is_some() {
                        f.auth_method = AuthMethod::VaultKey;
                    }
                }
                Task::none()
            }
            ManagerMsg::FormPrivateKeyPathChanged(v) => {
                if let Some(f) = &mut self.form {
                    f.private_key_path = v;
                }
                Task::none()
            }
            ManagerMsg::FormBrowsePrivateKey => Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .set_title("Select SSH Private Key")
                        .pick_file()
                        .await
                        .map(|h| h.path().display().to_string())
                },
                Message::FormPrivateKeyPicked,
            ),
            ManagerMsg::FormPassphraseChanged(v) => {
                if let Some(f) = &mut self.form {
                    f.passphrase = v;
                }
                Task::none()
            }
            ManagerMsg::FormToggleShowPassphrase => {
                if let Some(f) = &mut self.form {
                    f.show_passphrase = !f.show_passphrase;
                }
                Task::none()
            }
            ManagerMsg::FormZerotierChanged(v) => {
                if let Some(f) = &mut self.form {
                    f.zerotier = v;
                }
                Task::none()
            }
            ManagerMsg::FormSelectProxyJump(pick) => {
                if let Some(f) = &mut self.form {
                    f.proxy_jump_id = pick.id;
                }
                Task::none()
            }
            ManagerMsg::FormSelectGroup(pick) => {
                if let Some(f) = &mut self.form {
                    f.group_id = pick.id;
                }
                Task::none()
            }
            ManagerMsg::FormSelectIdentity(pick) => {
                if let Some(f) = &mut self.form {
                    f.identity_id = pick.id;
                }
                Task::none()
            }
            ManagerMsg::FormTagsChanged(v) => {
                if let Some(f) = &mut self.form {
                    f.tags = v;
                }
                Task::none()
            }
            ManagerMsg::FormForwardAgentToggled(v) => {
                if let Some(f) = &mut self.form {
                    f.forward_agent = v;
                }
                Task::none()
            }
            ManagerMsg::FormAutoLogToggled(v) => {
                if let Some(f) = &mut self.form {
                    f.auto_log = v;
                }
                Task::none()
            }
            ManagerMsg::FormKeepaliveChanged(v) => {
                if let Some(f) = &mut self.form {
                    f.keepalive_secs = v;
                }
                Task::none()
            }
            ManagerMsg::FormStartupCommandChanged(v) => {
                if let Some(f) = &mut self.form {
                    f.startup_command = v;
                }
                Task::none()
            }
            ManagerMsg::FormEnvVarsChanged(v) => {
                if let Some(f) = &mut self.form {
                    f.env_vars = v;
                }
                Task::none()
            }
            ManagerMsg::FormPortKnockChanged(v) => {
                if let Some(f) = &mut self.form {
                    f.port_knock = v;
                }
                Task::none()
            }
            ManagerMsg::SubmitForm => {
                if let Some(f) = &mut self.form {
                    match f.to_input() {
                        Ok(input) => {
                            let res = self.vault.lock().map_err(|_| anyhow::anyhow!("Vault busy")).and_then(|mut v| {
                                if let Some(id) = input.id.clone() {
                                    v.edit_server(&id, input)
                                } else {
                                    v.add_server(input).map(|_| ())
                                }
                            });
                            match res {
                                Ok(()) => {
                                    self.form = None;
                                    self.refresh_vault_cache();
                                }
                                Err(e) => f.error = Some(e.to_string()),
                            }
                        }
                        Err(err) => f.error = Some(err),
                    }
                }
                Task::none()
            }
        }
    }

    fn handle_modal_msg(&mut self, msg: ModalMsg) -> Task<Message> {
        if let Some(task) = self.handle_settings_termius_msg(msg.clone()) {
            return task;
        }
        match msg {
            ModalMsg::Noop => Task::none(),

            ModalMsg::Close => {
                self.modal = None;
                self.connect_started_at = None;
                Task::none()
            }

            ModalMsg::ConfirmDelete(server_id) => {
                self.with_vault(|v| {
                    let _ = v.delete_server(&server_id);
                });
                if matches!(&self.form, Some(f) if f.editing_id.as_deref() == Some(server_id.as_str()))
                {
                    self.form = None;
                }
                self.modal = None;
                self.refresh_vault_cache();
                Task::none()
            }

            ModalMsg::ExportToggleServer(id, checked) => {
                if let Some(ManagerModal::Export(st)) = &mut self.modal {
                    if checked {
                        st.selected_ids.insert(id);
                    } else {
                        st.selected_ids.remove(&id);
                    }
                }
                Task::none()
            }
            ModalMsg::ExportSelectAll => {
                if let Some(ManagerModal::Export(st)) = &mut self.modal {
                    st.selected_ids = self.servers.iter().map(|s| s.id.clone()).collect();
                }
                Task::none()
            }
            ModalMsg::ExportDeselectAll => {
                if let Some(ManagerModal::Export(st)) = &mut self.modal {
                    st.selected_ids.clear();
                }
                Task::none()
            }
            ModalMsg::ExportPasswordChanged(pw) => {
                if let Some(ManagerModal::Export(st)) = &mut self.modal {
                    st.password = pw;
                    st.error = None;
                }
                Task::none()
            }
            ModalMsg::ExportToggleShowPassword => {
                if let Some(ManagerModal::Export(st)) = &mut self.modal {
                    st.show_password = !st.show_password;
                }
                Task::none()
            }
            ModalMsg::ExportSubmit => {
                let Some(ManagerModal::Export(st)) = &mut self.modal else {
                    return Task::none();
                };
                if st.selected_ids.is_empty() {
                    st.error = Some("Select at least one server to export.".into());
                    return Task::none();
                }
                if st.password.is_empty() {
                    st.error = Some("Master encryption password is required.".into());
                    return Task::none();
                }
                let ids: Vec<String> = st.selected_ids.iter().cloned().collect();
                let pw = st.password.clone();
                let enc_res = self
                    .vault
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Vault busy"))
                    .and_then(|v| v.export_servers_encrypted(&pw, Some(&ids)));
                match enc_res {
                    Ok(encrypted_payload) => {
                        st.busy = true;
                        st.error = None;
                        st.status = None;
                        Task::perform(
                            async move {
                                let Some(handle) = rfd::AsyncFileDialog::new()
                                    .set_title("Export Glyph Encrypted Backup")
                                    .add_filter("Glyph Encrypted Backup", &["glyph"])
                                    .set_file_name("glyph-servers.glyph")
                                    .save_file()
                                    .await
                                else {
                                    return Ok(None);
                                };
                                let path = handle.path().to_path_buf();
                                tokio::fs::write(&path, encrypted_payload.as_bytes())
                                    .await
                                    .map_err(|e| format!("Failed to write backup file: {e}"))?;
                                Ok(Some(path.display().to_string()))
                            },
                            Message::ExportFinished,
                        )
                    }
                    Err(e) => {
                        st.error = Some(e.to_string());
                        Task::none()
                    }
                }
            }

            ModalMsg::ImportBrowseFile => Task::perform(
                async {
                    let Some(handle) = rfd::AsyncFileDialog::new()
                        .set_title("Open Glyph Encrypted Backup (.glyph)")
                        .add_filter("Glyph Backup", &["glyph"])
                        .pick_file()
                        .await
                    else {
                        return Ok(None);
                    };
                    let path = handle.path().to_path_buf();
                    let content = tokio::fs::read_to_string(&path)
                        .await
                        .map_err(|e| format!("Failed to read .glyph file: {e}"))?;
                    Ok(Some((path.display().to_string(), content)))
                },
                Message::ImportFileLoaded,
            ),
            ModalMsg::ImportPasswordChanged(pw) => {
                if let Some(ManagerModal::Import(st)) = &mut self.modal {
                    st.password = pw;
                    st.error = None;
                }
                Task::none()
            }
            ModalMsg::ImportToggleShowPassword => {
                if let Some(ManagerModal::Import(st)) = &mut self.modal {
                    st.show_password = !st.show_password;
                }
                Task::none()
            }
            ModalMsg::ImportDecryptSubmit => {
                let Some(ManagerModal::Import(st)) = &mut self.modal else {
                    return Task::none();
                };
                let Some(content) = st.encrypted_content.clone() else {
                    st.error = Some("Choose a .glyph backup file first.".into());
                    return Task::none();
                };
                let pw = st.password.clone();
                let dec_res = self.with_vault(|v| v.decode_import_servers(&content, &pw));
                match dec_res {
                    Ok(list) => {
                        if let Some(ManagerModal::Import(st)) = &mut self.modal {
                            st.selected_indices = (0..list.len()).collect();
                            st.step = ImportStep::SelectServers(list);
                            st.error = None;
                        }
                    }
                    Err(e) => {
                        if let Some(ManagerModal::Import(st)) = &mut self.modal {
                            st.error = Some(e.to_string());
                        }
                    }
                }
                Task::none()
            }
            ModalMsg::ImportBackToUnlock => {
                if let Some(ManagerModal::Import(st)) = &mut self.modal {
                    st.step = ImportStep::UnlockFile;
                    st.selected_indices.clear();
                    st.error = None;
                }
                Task::none()
            }
            ModalMsg::ImportToggleServer(idx, checked) => {
                if let Some(ManagerModal::Import(st)) = &mut self.modal {
                    if checked {
                        st.selected_indices.insert(idx);
                    } else {
                        st.selected_indices.remove(&idx);
                    }
                }
                Task::none()
            }
            ModalMsg::ImportSelectAll => {
                if let Some(ManagerModal::Import(st)) = &mut self.modal {
                    if let ImportStep::SelectServers(list) = &st.step {
                        st.selected_indices = (0..list.len()).collect();
                    }
                }
                Task::none()
            }
            ModalMsg::ImportDeselectAll => {
                if let Some(ManagerModal::Import(st)) = &mut self.modal {
                    st.selected_indices.clear();
                }
                Task::none()
            }
            ModalMsg::ImportConfirmSubmit => {
                let chosen_opt = match &self.modal {
                    Some(ManagerModal::Import(st)) => match &st.step {
                        ImportStep::SelectServers(list) => Some(
                            list.iter()
                                .enumerate()
                                .filter(|(i, _)| st.selected_indices.contains(i))
                                .map(|(_, s)| s.clone())
                                .collect::<Vec<ServerEntry>>(),
                        ),
                        _ => None,
                    },
                    _ => return Task::none(),
                };
                if let Some(chosen) = chosen_opt {
                    let res = self.with_vault(|v| v.import_servers(chosen));
                    match res {
                        Ok(_) => {
                            self.modal = None;
                            self.refresh_vault_cache();
                        }
                        Err(e) => {
                            if let Some(ManagerModal::Import(st)) = &mut self.modal {
                                st.error = Some(e.to_string());
                            }
                        }
                    }
                }
                Task::none()
            }

            // SSH Key Vault
            ModalMsg::SshKeyToggleGenerate => {
                if let Some(ManagerModal::SshKeys(st)) = &mut self.modal {
                    st.show_generate = !st.show_generate;
                    if st.show_generate {
                        st.show_import = false;
                    }
                    st.error = None;
                    st.status = None;
                }
                Task::none()
            }
            ModalMsg::SshKeyToggleImport => {
                if let Some(ManagerModal::SshKeys(st)) = &mut self.modal {
                    st.show_import = !st.show_import;
                    if st.show_import {
                        st.show_generate = false;
                    }
                    st.error = None;
                    st.status = None;
                }
                Task::none()
            }
            ModalMsg::SshKeyGenName(v) => {
                if let Some(ManagerModal::SshKeys(st)) = &mut self.modal {
                    st.gen_name = v;
                }
                Task::none()
            }
            ModalMsg::SshKeyGenAlgo(a) => {
                if let Some(ManagerModal::SshKeys(st)) = &mut self.modal {
                    st.gen_algo = a;
                }
                Task::none()
            }
            ModalMsg::SshKeyGenPass(v) => {
                if let Some(ManagerModal::SshKeys(st)) = &mut self.modal {
                    st.gen_passphrase = v;
                }
                Task::none()
            }
            ModalMsg::SshKeyGenToggleShowPass => {
                if let Some(ManagerModal::SshKeys(st)) = &mut self.modal {
                    st.show_gen_pass = !st.show_gen_pass;
                }
                Task::none()
            }
            ModalMsg::SshKeyGenSubmit => {
                let (name, algo, pass_str) = match &self.modal {
                    Some(ManagerModal::SshKeys(st)) => {
                        (st.gen_name.trim().to_string(), st.gen_algo, st.gen_passphrase.clone())
                    }
                    _ => return Task::none(),
                };
                if name.is_empty() {
                    if let Some(ManagerModal::SshKeys(st)) = &mut self.modal {
                        st.error = Some("Enter a label for the new SSH key.".into());
                    }
                    return Task::none();
                }
                let res = self.with_vault(|v| {
                    let pass = if pass_str.is_empty() { None } else { Some(pass_str.as_str()) };
                    v.generate_ssh_key(&name, algo, pass)
                });
                match res {
                    Ok(created) => {
                        if let Some(ManagerModal::SshKeys(st)) = &mut self.modal {
                            st.gen_name.clear();
                            st.gen_passphrase.clear();
                            st.show_generate = false;
                            st.error = None;
                            st.status = Some(format!(
                                "Generated {} key \"{}\".",
                                created.algorithm, created.name
                            ));
                        }
                        self.refresh_vault_cache();
                    }
                    Err(e) => {
                        if let Some(ManagerModal::SshKeys(st)) = &mut self.modal {
                            st.error = Some(e.to_string());
                        }
                    }
                }
                Task::none()
            }
            ModalMsg::SshKeyImpName(v) => {
                if let Some(ManagerModal::SshKeys(st)) = &mut self.modal {
                    st.imp_name = v;
                }
                Task::none()
            }
            ModalMsg::SshKeyImpBrowseFile => Task::perform(
                async {
                    let Some(handle) = rfd::AsyncFileDialog::new()
                        .set_title("Import OpenSSH Private Key")
                        .pick_file()
                        .await
                    else {
                        return Ok(None);
                    };
                    let path = handle.path().to_path_buf();
                    let pem = tokio::fs::read_to_string(&path)
                        .await
                        .map_err(|e| format!("Failed to read SSH key file: {e}"))?;
                    Ok(Some((path.display().to_string(), pem)))
                },
                Message::SshKeyFileLoaded,
            ),
            ModalMsg::SshKeyImpPass(v) => {
                if let Some(ManagerModal::SshKeys(st)) = &mut self.modal {
                    st.imp_passphrase = v;
                }
                Task::none()
            }
            ModalMsg::SshKeyImpToggleShowPass => {
                if let Some(ManagerModal::SshKeys(st)) = &mut self.modal {
                    st.show_imp_pass = !st.show_imp_pass;
                }
                Task::none()
            }
            ModalMsg::SshKeyImpSubmit => {
                let (name, pem_opt, pass_str) = match &self.modal {
                    Some(ManagerModal::SshKeys(st)) => (
                        st.imp_name.trim().to_string(),
                        st.imp_pem.clone(),
                        st.imp_passphrase.clone(),
                    ),
                    _ => return Task::none(),
                };
                if name.is_empty() {
                    if let Some(ManagerModal::SshKeys(st)) = &mut self.modal {
                        st.error = Some("Enter a label for the imported SSH key.".into());
                    }
                    return Task::none();
                }
                let Some(pem) = pem_opt else {
                    if let Some(ManagerModal::SshKeys(st)) = &mut self.modal {
                        st.error = Some("Choose an OpenSSH private key file first.".into());
                    }
                    return Task::none();
                };
                let res = self.with_vault(|v| {
                    let pass = if pass_str.is_empty() { None } else { Some(pass_str.as_str()) };
                    v.import_ssh_key(&name, &pem, pass)
                });
                match res {
                    Ok(imported) => {
                        if let Some(ManagerModal::SshKeys(st)) = &mut self.modal {
                            st.imp_name.clear();
                            st.imp_path = None;
                            st.imp_pem = None;
                            st.imp_passphrase.clear();
                            st.show_import = false;
                            st.error = None;
                            st.status = Some(format!(
                                "Imported {} key \"{}\".",
                                imported.algorithm, imported.name
                            ));
                        }
                        self.refresh_vault_cache();
                    }
                    Err(e) => {
                        if let Some(ManagerModal::SshKeys(st)) = &mut self.modal {
                            st.error = Some(e.to_string());
                        }
                    }
                }
                Task::none()
            }
            ModalMsg::SshKeyCopyPublic(kid) => {
                if let Some(pub_key) = self.with_vault(|v| v.get_ssh_key_public(&kid)) {
                    clipboard_set(pub_key);
                    if let Some(ManagerModal::SshKeys(st)) = &mut self.modal {
                        st.copied_key_id = Some(kid);
                        st.error = None;
                        st.status = Some("Public key copied to clipboard.".into());
                    }
                }
                Task::none()
            }
            ModalMsg::SshKeyToggleDeploy(kid) => {
                if let Some(ManagerModal::SshKeys(st)) = &mut self.modal {
                    if st.deploy_key_id.as_deref() == Some(kid.as_str()) {
                        st.deploy_key_id = None;
                    } else {
                        st.deploy_key_id = Some(kid);
                        if st.deploy_server_id.is_none() {
                            st.deploy_server_id = self.servers.first().map(|s| s.id.clone());
                        }
                    }
                    st.error = None;
                }
                Task::none()
            }
            ModalMsg::SshKeySelectDeployServer(opt) => {
                if let Some(ManagerModal::SshKeys(st)) = &mut self.modal {
                    st.deploy_server_id = Some(opt.id);
                }
                Task::none()
            }
            ModalMsg::SshKeyDeploySubmit => {
                let (key_id, server_id) = match &self.modal {
                    Some(ManagerModal::SshKeys(st)) => match (&st.deploy_key_id, &st.deploy_server_id) {
                        (Some(k), Some(s)) => (k.clone(), s.clone()),
                        _ => return Task::none(),
                    },
                    _ => return Task::none(),
                };

                let deploy_data = self.with_vault(|v| {
                    let pub_key = v
                        .get_ssh_key_public(&key_id)
                        .ok_or_else(|| "SSH key not found.".to_string())?;
                    let cfg = v.get_connection_config(&server_id).map_err(|e| e.to_string())?;
                    let known_hosts = v.known_hosts_map();
                    let label = v
                        .get_server(&server_id)
                        .map(|s| display_server_name(&s))
                        .unwrap_or_else(|| format!("{}:{}", cfg.host, cfg.port));
                    Ok::<_, String>((pub_key, cfg, known_hosts, label))
                });

                let (pub_key, cfg, known_hosts, server_label) = match deploy_data {
                    Ok(d) => d,
                    Err(e) => {
                        if let Some(ManagerModal::SshKeys(st)) = &mut self.modal {
                            st.error = Some(e);
                        }
                        return Task::none();
                    }
                };

                let host_port = format!("{}:{}", cfg.host, cfg.port);
                if !known_hosts.contains_key(&host_port) {
                    if let Some(ManagerModal::SshKeys(st)) = &mut self.modal {
                        st.error = Some(format!(
                            "Host key for {host_port} is not trusted yet. Connect to this server once first to verify TOFU host identity, then deploy."
                        ));
                    }
                    return Task::none();
                }

                if let Some(ManagerModal::SshKeys(st)) = &mut self.modal {
                    st.busy = true;
                    st.error = None;
                    st.status = Some(format!("Connecting to {server_label} to authorize key..."));
                }

                let bus = app_bus_sender();
                tokio::spawn(async move {
                    let (ev_tx, mut ev_rx) = mpsc::unbounded_channel();
                    let (hk_tx, mut hk_rx) = mpsc::unbounded_channel::<HostKeyPrompt>();
                    tokio::spawn(async move {
                        while let Some(p) = hk_rx.recv().await {
                            let _ = p.responder.send(false);
                        }
                    });
                    tokio::spawn(async move { while ev_rx.recv().await.is_some() {} });

                    let res = match SshSession::connect(cfg, known_hosts, ev_tx, hk_tx).await {
                        Ok(outcome) => {
                            let dep = outcome.session.deploy_public_key(&pub_key).await;
                            outcome.session.disconnect().await;
                            dep.map(|()| {
                                format!(
                                    "Authorized SSH key on {server_label} (~/.ssh/authorized_keys) and enabled Vault Key auth."
                                )
                            })
                            .map_err(|e| e.to_string())
                        }
                        Err(e) => Err(e.to_string()),
                    };

                    let _ = bus.send(BusEvent::SshKeyDeployed {
                        key_id,
                        server_id,
                        result: res,
                    });
                });
                Task::none()
            }
            ModalMsg::SshKeyDelete(kid) => {
                self.with_vault(|v| {
                    let _ = v.delete_ssh_key(&kid);
                });
                self.refresh_vault_cache();
                if let Some(ManagerModal::SshKeys(st)) = &mut self.modal {
                    if st.deploy_key_id.as_deref() == Some(kid.as_str()) {
                        st.deploy_key_id = None;
                    }
                    st.status = Some("SSH key deleted from Vault.".into());
                }
                Task::none()
            }

            ModalMsg::RespondHostKey(pid, accept) => {
                if let Ok(mut map) = responders_map().lock() {
                    if let Some(tx) = map.remove(&pid) {
                        let _ = tx.send(accept);
                    }
                }
                if let Some(ManagerModal::Connecting(st)) = &mut self.modal {
                    st.host_key_prompt = None;
                    st.logs.push(if accept {
                        "Host key accepted by user — continuing handshake...".into()
                    } else {
                        "Host key rejected by user — aborting connection.".into()
                    });
                }
                Task::none()
            }

            ModalMsg::ConnectingRetry => {
                if let Some(ManagerModal::Connecting(st)) = &self.modal {
                    let sid = st.server_id.clone();
                    self.start_connect(sid);
                }
                Task::none()
            }

            ModalMsg::SelectSettingsTab(tab) => {
                if let Some(ManagerModal::Settings(st)) = &mut self.modal {
                    st.active_tab = tab;
                    st.mcp_status = None;
                    st.mcp_error = None;
                }
                Task::none()
            }
            ModalMsg::RemoveKnownHost(host_port) => {
                self.with_vault(|v| {
                    let _ = v.remove_known_host(&host_port);
                });
                self.refresh_vault_cache();
                Task::none()
            }
            ModalMsg::RefreshMcpClients => {
                if let Some(ManagerModal::Settings(st)) = &mut self.modal {
                    st.clients = scan_mcp_clients();
                    st.mcp_error = None;
                }
                Task::none()
            }
            ModalMsg::AutoInstallMcp(client_id) => {
                if let Some(ManagerModal::Settings(st)) = &mut self.modal {
                    let bin = PathBuf::from(&st.mcp_bin_path);
                    match auto_install_mcp_client(client_id, &bin) {
                        Ok(written) => {
                            st.clients = scan_mcp_clients();
                            st.mcp_error = None;
                            st.mcp_status = Some(format!(
                                "Configured {} ({})",
                                client_id.display_name(),
                                written.display()
                            ));
                        }
                        Err(e) => {
                            st.mcp_status = None;
                            st.mcp_error = Some(e.to_string());
                        }
                    }
                }
                Task::none()
            }
            ModalMsg::ToggleMcpSnippet(cid) => {
                if let Some(ManagerModal::Settings(st)) = &mut self.modal {
                    st.expanded_snippet = if st.expanded_snippet == Some(cid) {
                        None
                    } else {
                        Some(cid)
                    };
                }
                Task::none()
            }
            ModalMsg::CopyMcpSnippet(cid) => {
                if let Some(ManagerModal::Settings(st)) = &mut self.modal {
                    if let Some(info) = st.clients.iter().find(|c| c.id == cid) {
                        let snippet = build_config_snippet(
                            info.format,
                            None,
                            Path::new(&st.mcp_bin_path),
                        );
                        clipboard_set(snippet);
                        st.copied_snippet = Some(cid);
                    }
                }
                Task::none()
            }
            ModalMsg::ToggleCustomMcp => {
                if let Some(ManagerModal::Settings(st)) = &mut self.modal {
                    st.show_custom = !st.show_custom;
                }
                Task::none()
            }
            ModalMsg::CustomMcpPathChanged(p) => {
                if let Some(ManagerModal::Settings(st)) = &mut self.modal {
                    st.custom_path = p;
                }
                Task::none()
            }
            ModalMsg::BrowseCustomMcpPath => Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .set_title("Select AI Client JSON Config")
                        .add_filter("JSON Config", &["json"])
                        .pick_file()
                        .await
                        .map(|h| h.path().display().to_string())
                },
                Message::CustomMcpPathPicked,
            ),
            ModalMsg::CustomMcpFormatChanged(fmt) => {
                if let Some(ManagerModal::Settings(st)) = &mut self.modal {
                    st.custom_format = fmt;
                }
                Task::none()
            }
            ModalMsg::CustomMcpKeyChanged(k) => {
                if let Some(ManagerModal::Settings(st)) = &mut self.modal {
                    st.custom_key = k;
                }
                Task::none()
            }
            ModalMsg::WriteCustomMcp => {
                if let Some(ManagerModal::Settings(st)) = &mut self.modal {
                    let target = st.custom_path.trim().to_string();
                    if target.is_empty() {
                        st.mcp_error = Some("Enter or browse a JSON config file path first.".into());
                        return Task::none();
                    }
                    let bin = PathBuf::from(&st.mcp_bin_path);
                    match write_mcp_config_at(
                        Path::new(&target),
                        st.custom_format,
                        Some(&st.custom_key),
                        &bin,
                    ) {
                        Ok(()) => {
                            st.clients = scan_mcp_clients();
                            st.mcp_error = None;
                            st.mcp_status = Some(format!("Wrote glyph_mcp entry into {target}"));
                        }
                        Err(e) => {
                            st.mcp_status = None;
                            st.mcp_error = Some(e.to_string());
                        }
                    }
                }
                Task::none()
            }
            ModalMsg::OpenMobileFromSettings => self.open_mobile_modal(),
            ModalMsg::CheckUpdates => self.trigger_manual_update_check(),
            ModalMsg::OpenUpdateModal => {
                self.modal = None;
                self.updater.modal_open = true;
                Task::none()
            }
            ModalMsg::StartUpdateDownload => {
                self.modal = None;
                self.start_update_download()
            }
            ModalMsg::InstallDownloadedUpdate => {
                self.modal = None;
                self.install_downloaded_update()
            }
            ModalMsg::OpenUpdateReleaseUrl(url) => {
                titlebar::open_url_in_browser(&url);
                Task::none()
            }

            // Mobile APK QR Modal
            ModalMsg::SelectMobileQrMode(mode) => {
                if let Some(ManagerModal::Mobile(st)) = &mut self.modal {
                    st.qr_mode = mode;
                    st.copied = false;
                    st.refresh_qr();
                }
                Task::none()
            }
            ModalMsg::CopyMobileUrl => {
                if let Some(ManagerModal::Mobile(st)) = &mut self.modal {
                    clipboard_set(st.active_url().to_string());
                    st.copied = true;
                }
                Task::none()
            }
            ModalMsg::OpenMobileUrl => {
                if let Some(ManagerModal::Mobile(st)) = &self.modal {
                    titlebar::open_url_in_browser(st.active_url());
                }
                Task::none()
            }
            _ => Task::none(),
        }
    }

    fn handle_sidebar_msg(&mut self, wid: window::Id, msg: SidebarMsg) -> Task<Message> {
        match msg {
            SidebarMsg::SelectTab(tab) => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    srv.active_tab = tab;
                    match tab {
                        ServerTab::Terminal => {
                            let active = srv.terminal.active_tab_id.clone();
                            srv.terminal.focused_tab_id = active;
                        }
                        ServerTab::Tunnels => {
                            spawn_list_tunnels(wid, srv.session.clone());
                        }
                        ServerTab::Containers
                            if !srv.containers.loaded_once && !srv.containers.loading =>
                        {
                            srv.containers.loading = true;
                            spawn_list_containers(wid, srv.session.clone());
                        }
                        _ => {}
                    }
                }
                Task::none()
            }
            SidebarMsg::OpenMobile => Task::batch([self.open_mobile_modal(), self.show_manager_window()]),
            SidebarMsg::Disconnect => self.handle_window_close(wid),
            SidebarMsg::OpenGithub => {
                titlebar::open_url_in_browser("https://github.com/TheLunatic1");
                Task::none()
            }
        }
    }

    fn handle_terminal_msg(&mut self, wid: window::Id, msg: TerminalPageMsg) -> Task<Message> {
        let Some(srv) = self.server_windows.get_mut(&wid) else {
            return Task::none();
        };

        match msg {
            TerminalPageMsg::NewTab => {
                let tid = srv.terminal.add_tab();
                terminal_page::spawn_open_shell(srv.session.clone(), tid, 100, 30, wid);
            }
            TerminalPageMsg::SelectTab(tid) => {
                srv.terminal.active_tab_id = tid.clone();
                srv.terminal.focused_tab_id = tid;
            }
            TerminalPageMsg::CloseTab(tid) => {
                if srv.terminal.tabs.len() > 1 {
                    let sess = srv.session.clone();
                    let tid_c = tid.clone();
                    tokio::spawn(async move {
                        sess.close_shell(&tid_c).await;
                    });
                    srv.terminal.tabs.retain(|t| t.id != tid);
                    if srv.terminal.active_tab_id == tid {
                        srv.terminal.active_tab_id = srv
                            .terminal
                            .tabs
                            .first()
                            .map(|t| t.id.clone())
                            .unwrap_or_default();
                    }
                    if srv.terminal.split_tab_id.as_deref() == Some(tid.as_str()) {
                        srv.terminal.split_tab_id = None;
                    }
                    srv.terminal.focused_tab_id = srv.terminal.active_tab_id.clone();
                }
            }
            TerminalPageMsg::ToggleSplit => {
                if srv.terminal.split_tab_id.is_some() {
                    srv.terminal.split_tab_id = None;
                    srv.terminal.focused_tab_id = srv.terminal.active_tab_id.clone();
                } else if let Some(other_id) = srv
                    .terminal
                    .tabs
                    .iter()
                    .find(|t| t.id != srv.terminal.active_tab_id)
                    .map(|t| t.id.clone())
                {
                    srv.terminal.split_tab_id = Some(other_id.clone());
                    srv.terminal.focused_tab_id = other_id;
                } else {
                    let prev_active = srv.terminal.active_tab_id.clone();
                    let new_id = srv.terminal.add_tab();
                    srv.terminal.active_tab_id = prev_active;
                    srv.terminal.split_tab_id = Some(new_id.clone());
                    srv.terminal.focused_tab_id = new_id.clone();
                    terminal_page::spawn_open_shell(srv.session.clone(), new_id, 50, 30, wid);
                }
            }
            TerminalPageMsg::SelectTheme(theme_id) => {
                srv.terminal.theme = theme_id;
            }
            TerminalPageMsg::FontSmaller => {
                srv.terminal.font_size = (srv.terminal.font_size - 1.0).max(10.0);
            }
            TerminalPageMsg::FontLarger => {
                srv.terminal.font_size = (srv.terminal.font_size + 1.0).min(26.0);
            }
            TerminalPageMsg::ToggleSearch => {
                srv.terminal.show_search = !srv.terminal.show_search;
                if !srv.terminal.show_search {
                    srv.terminal.search_text.clear();
                    if let Some(tab) = srv.terminal.active_tab_mut() {
                        tab.emulator.clear_search();
                    }
                }
            }
            TerminalPageMsg::SearchInput(q) => {
                srv.terminal.search_text = q.clone();
                if let Some(tab) = srv.terminal.active_tab_mut() {
                    if q.is_empty() {
                        tab.emulator.clear_search();
                    } else {
                        tab.emulator.search(&q, false);
                    }
                }
            }
            TerminalPageMsg::SearchNext => {
                let q = srv.terminal.search_text.clone();
                if let Some(tab) = srv.terminal.active_tab_mut() {
                    if !q.is_empty() {
                        tab.emulator.search(&q, true);
                    }
                }
            }
            TerminalPageMsg::SearchPrev => {
                let q = srv.terminal.search_text.clone();
                if let Some(tab) = srv.terminal.active_tab_mut() {
                    if !q.is_empty() {
                        tab.emulator.search(&q, false);
                    }
                }
            }
            TerminalPageMsg::ClearActive => {
                if let Some(tab) = srv.terminal.active_tab_mut() {
                    tab.emulator.clear();
                }
            }
            TerminalPageMsg::ToggleSnippets => {
                srv.terminal.show_snippets = !srv.terminal.show_snippets;
                if srv.terminal.show_snippets {
                    srv.terminal.show_secrets = false;
                }
            }
            TerminalPageMsg::ToggleSecrets => {
                srv.terminal.show_secrets = !srv.terminal.show_secrets;
                if srv.terminal.show_secrets {
                    srv.terminal.show_snippets = false;
                }
            }
            TerminalPageMsg::CycleBroadcast => {
                srv.terminal.broadcast_mode = srv.terminal.broadcast_mode.next();
            }
            TerminalPageMsg::ToggleHighlights => {
                srv.terminal.highlights_enabled = !srv.terminal.highlights_enabled;
            }
            TerminalPageMsg::ToggleRecording => {
                let active_id = srv.terminal.active_tab_id.clone();
                if let Some(rec) = srv.terminal.recorders.remove(&active_id) {
                    srv.terminal.notice = Some(format!(
                        "Saved .cast recording -> {}",
                        rec.path().display()
                    ));
                } else {
                    let label = format!("{}-{}", display_server_name(&srv.server), active_id);
                    let (cols, rows) = srv
                        .terminal
                        .tabs
                        .iter()
                        .find(|t| t.id == active_id)
                        .map(|t| (t.emulator.cols() as u16, t.emulator.rows() as u16))
                        .unwrap_or((100, 30));
                    match glyph_terminal::SessionRecorder::start(&label, u32::from(cols), u32::from(rows)) {
                        Ok(rec) => {
                            srv.terminal.notice = Some(format!(
                                "Recording .cast -> {}",
                                rec.path().display()
                            ));
                            srv.terminal.recorders.insert(active_id, rec);
                        }
                        Err(e) => srv.terminal.notice = Some(format!("Record error: {e}")),
                    }
                }
            }
            TerminalPageMsg::ExportPlainLog => {
                let active_id = srv.terminal.active_tab_id.clone();
                let label = format!("{}-{}", display_server_name(&srv.server), active_id);
                if let Some(tab) = srv.terminal.tabs.iter().find(|t| t.id == active_id) {
                    let body = tab.emulator.all_text();
                    match glyph_terminal::export_plain_log(&label, &body) {
                        Ok(p) => {
                            srv.terminal.notice =
                                Some(format!("Exported plain log -> {}", p.display()));
                        }
                        Err(e) => srv.terminal.notice = Some(format!("Log export error: {e}")),
                    }
                }
            }
            TerminalPageMsg::RunSnippet(cmd) => {
                srv.terminal.show_snippets = false;
                self.trigger_snippet_or_prompt(wid, None, "Quick Snippet".to_string(), cmd);
                return Task::none();
            }
            TerminalPageMsg::InjectSecret(secret_id) => {
                srv.terminal.show_secrets = false;
                let secret_val = self
                    .vault
                    .lock()
                    .ok()
                    .and_then(|v| v.get_secret_value(&secret_id).ok().flatten());
                if let Some(val) = secret_val {
                    let target = srv.terminal.focused_tab_id.clone();
                    srv.terminal.send_input(&target, val.into_bytes());
                }
            }
            TerminalPageMsg::CanvasAction(tid, ev) => match ev {
                TerminalEvent::Input(bytes) => {
                    self.send_terminal_payload(wid, &tid, bytes);
                    return Task::none();
                }
                TerminalEvent::Resized { cols, rows } => {
                    srv.terminal.resize_pane(&tid, cols, rows);
                }
                TerminalEvent::CopyText(txt) => {
                    clipboard_set(txt);
                }
                TerminalEvent::PasteRequested => {
                    if let Some(txt) = clipboard_get() {
                        let normalized = txt.replace("\r\n", "\r").replace('\n', "\r");
                        self.send_terminal_payload(wid, &tid, normalized.into_bytes());
                        return Task::none();
                    }
                }
                TerminalEvent::ToggleSearch => {
                    srv.terminal.show_search = !srv.terminal.show_search;
                }
                TerminalEvent::FocusRequested => {
                    srv.terminal.focused_tab_id = tid;
                }
                TerminalEvent::StartSelection { col, row } => {
                    srv.terminal.focused_tab_id = tid.clone();
                    if let Some(tab) = srv.terminal.tabs.iter_mut().find(|t| t.id == tid) {
                        tab.emulator.start_selection(col, row);
                    }
                }
                TerminalEvent::UpdateSelection { col, row } => {
                    if let Some(tab) = srv.terminal.tabs.iter_mut().find(|t| t.id == tid) {
                        tab.emulator.update_selection(col, row);
                    }
                }
                TerminalEvent::WheelScroll(delta) => {
                    if let Some(tab) = srv.terminal.tabs.iter_mut().find(|t| t.id == tid) {
                        tab.emulator.scroll_display(delta);
                    }
                }
            },
        }

        Task::none()
    }

    pub(crate) fn start_connect(&mut self, server_id: String) {
        let info_res = self.with_vault(|v| {
            let Some(summary) = v.get_server(&server_id) else {
                return Ok(None);
            };
            let cfg = v.get_connection_config(&server_id).map_err(|e| e.to_string())?;
            let known_hosts = v.known_hosts_map();
            Ok::<_, String>(Some((summary, cfg, known_hosts)))
        });
        let (summary, cfg, known_hosts) = match info_res {
            Ok(Some(d)) => d,
            Ok(None) => return,
            Err(e) => {
                self.disconnect_banner = Some(e);
                return;
            }
        };

        self.connect_started_at = Some(Instant::now());
        self.modal = Some(ManagerModal::Connecting(ConnectingModalState {
            server_id: summary.id.clone(),
            server_name: display_server_name(&summary),
            host: summary.host.clone(),
            port: summary.port,
            username: summary.username.clone(),
            os: summary.os.clone(),
            elapsed_ms: 0,
            logs: vec![format!(
                "Preparing SSH connection to {}:{}...",
                summary.host, summary.port
            )],
            host_key_prompt: None,
            error: None,
            blink_on: true,
        }));

        let bus = app_bus_sender();
        tokio::spawn(async move {
            let (ev_tx, mut ev_rx) = mpsc::unbounded_channel::<SessionEvent>();
            let (hk_tx, mut hk_rx) = mpsc::unbounded_channel::<HostKeyPrompt>();
            let window_slot: Arc<Mutex<Option<window::Id>>> = Arc::new(Mutex::new(None));

            // Forward TOFU host key prompts onto global bus
            let bus_hk = bus.clone();
            let sid_hk = server_id.clone();
            tokio::spawn(async move {
                while let Some(prompt) = hk_rx.recv().await {
                    let prompt_id = PROMPT_SEQ.fetch_add(1, Ordering::Relaxed);
                    if let Ok(mut map) = responders_map().lock() {
                        map.insert(prompt_id, prompt.responder);
                    }
                    let _ = bus_hk.send(BusEvent::HostKeyPrompt {
                        server_id: sid_hk.clone(),
                        prompt_id,
                        host: prompt.host,
                        port: prompt.port,
                        algorithm: prompt.algorithm,
                        fingerprint_sha256: prompt.fingerprint_sha256,
                        mismatch_expected: prompt.mismatch_expected,
                    });
                }
            });

            // Forward status logs before window opens, then full SessionEvents afterward
            let bus_ev = bus.clone();
            let sid_ev = server_id.clone();
            let slot_ev = Arc::clone(&window_slot);
            tokio::spawn(async move {
                while let Some(ev) = ev_rx.recv().await {
                    let wid_opt = slot_ev.lock().ok().and_then(|g| *g);
                    if let Some(wid) = wid_opt {
                        let _ = bus_ev.send(BusEvent::Session(wid, ev));
                    } else if let SessionEvent::Status(line) = ev {
                        let _ = bus_ev.send(BusEvent::ConnectLog {
                            server_id: sid_ev.clone(),
                            line,
                        });
                    }
                }
            });

            match SshSession::connect(cfg, known_hosts, ev_tx.clone(), hk_tx).await {
                Ok(outcome) => {
                    let _ = bus.send(BusEvent::ConnectSucceeded {
                        server_id,
                        session: outcome.session,
                        detected_os: outcome.os,
                        newly_trusted: outcome.newly_trusted_hosts,
                        event_tx: ev_tx,
                        window_slot,
                    });
                }
                Err(e) => {
                    let _ = bus.send(BusEvent::ConnectFailed {
                        server_id,
                        error: e.to_string(),
                    });
                }
            }
        });
    }

    fn view(&self, wid: window::Id) -> Element<'_, Message> {
        if self.locked {
            return crate::ui::lock_screen::view(&self.lock_screen).map(Message::LockScreen);
        }
        if let Some(qt) = self.quick_windows.get(&wid) {
            let base = crate::ui::quick_terminal::view(qt, &self.compiled_highlights)
                .map(move |m| Message::QuickTerminal(wid, m));
            return if self.palette.open {
                stack![
                    base,
                    crate::ui::palette::view(&self.palette).map(Message::Palette),
                ]
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
            } else {
                base
            };
        }
        if let Some(srv) = self.server_windows.get(&wid) {
            return self.view_server_window(wid, srv);
        }

        let title_el = titlebar::view("Glyph", true).map(move |m| Message::TitleBar(wid, m));
        let base = manager::view(
            &self.servers,
            &self.groups,
            &self.identities,
            &self.ssh_keys,
            self.form.as_ref(),
            &self.manager_search,
            self.selected_group_filter.as_deref(),
            self.selected_tag_filter.as_deref(),
            self.app_lock_cfg.enabled,
            self.disconnect_banner.as_deref(),
            self.manager_size.width,
        )
        .map(Message::Manager);

        let mut layers: Vec<Element<'_, Message>> = vec![base];
        if let Some(tmodal) = &self.termius_modal {
            if self.termius_modal_window.is_none() || self.termius_modal_window == Some(wid) {
                layers.push(
                    crate::ui::termius_modals::view(
                        tmodal,
                        &self.identities,
                        &self.groups,
                        &self.servers,
                        &self.ssh_keys,
                    )
                    .map(Message::TermiusModalAction),
                );
            }
        }
        if let Some(modal) = &self.modal {
            let overlay = modals::view(
                modal,
                &self.servers,
                &self.ssh_keys,
                &self.known_hosts,
                &self.updater,
            )
            .map(Message::Modal);
            layers.push(overlay);
        }
        if self.updater.modal_open {
            layers.push(update_modal::view_modal(&self.updater).map(Message::Update));
        }
        if self.palette.open {
            layers.push(crate::ui::palette::view(&self.palette).map(Message::Palette));
        }

        if self.show_splash {
            let elapsed_ms = self
                .splash_started_at
                .map(|t| t.elapsed().as_millis() as u64)
                .unwrap_or(0);
            layers.push(splash::view(elapsed_ms, self.splash_alpha));
        }

        let body = stack(layers)
            .width(Length::Fill)
            .height(Length::Fill);

        let mut root = column![title_el];
        if let Some(banner) = update_modal::view_banner(&self.updater) {
            root = root.push(banner.map(Message::Update));
        }
        root.push(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_server_window<'a>(
        &'a self,
        wid: window::Id,
        srv: &'a ServerWindowState,
    ) -> Element<'a, Message> {
        let header_title = format!(
            "Glyph — {} ({}@{})",
            display_server_name(&srv.server),
            srv.server.username,
            srv.server.host
        );
        let title_el = titlebar::view(header_title, false).map(move |m| Message::TitleBar(wid, m));
        let sidebar_el = sidebar::view(srv.active_tab).map(move |m| Message::Sidebar(wid, m));
        let enc_avail = self
            .vault
            .lock()
            .map(|v| v.is_encryption_available())
            .unwrap_or(true);

        let peer_servers: Vec<(window::Id, String)> = self
            .server_windows
            .iter()
            .filter(|(&other_id, _)| other_id != wid)
            .map(|(&other_id, other)| (other_id, display_server_name(&other.server)))
            .collect();

        let main_pane: Element<'a, Message> = match srv.active_tab {
            ServerTab::Dashboard => dashboard::view(&srv.dashboard, &srv.server, srv.window_size.width)
                .map(move |m| Message::Dashboard(wid, m)),
            ServerTab::Terminal => terminal_page::view(
                &srv.terminal,
                &srv.cached_commands,
                &srv.cached_secrets,
                &self.compiled_highlights,
            )
            .map(move |m| Message::Terminal(wid, m)),
            ServerTab::Sftp => {
                sftp_page::view(&srv.sftp, peer_servers).map(move |m| Message::Sftp(wid, m))
            }
            ServerTab::Commands => commands_page::view(
                &srv.commands,
                &srv.cached_commands,
                &self.snippet_packages,
            )
            .map(move |m| Message::Commands(wid, m)),
            ServerTab::Secrets => secrets_page::view(&srv.secrets, &srv.cached_secrets, enc_avail)
                .map(move |m| Message::Secrets(wid, m)),
            ServerTab::Tunnels => {
                tunnels_page::view(&srv.tunnels, &srv.server.saved_tunnels)
                    .map(move |m| Message::Tunnels(wid, m))
            }
            ServerTab::Containers => {
                containers_page::view(&srv.containers).map(move |m| Message::Containers(wid, m))
            }
            ServerTab::Vnc => vnc_page::view(&srv.vnc).map(move |m| Message::Vnc(wid, m)),
        };

        let content_row = row![
            sidebar_el,
            container(main_pane)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(dark_900_bg),
        ]
        .width(Length::Fill)
        .height(Length::Fill);

        let mut base_col = column![title_el];
        if let Some(banner) = update_modal::view_banner(&self.updater) {
            base_col = base_col.push(banner.map(Message::Update));
        }
        let base_window: Element<'a, Message> = base_col
            .push(content_row)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        let mut layers: Vec<Element<'a, Message>> = vec![base_window];
        if let Some(tmodal) = &self.termius_modal {
            if self.termius_modal_window == Some(wid) {
                layers.push(
                    crate::ui::termius_modals::view(
                        tmodal,
                        &self.identities,
                        &self.groups,
                        &self.servers,
                        &self.ssh_keys,
                    )
                    .map(Message::TermiusModalAction),
                );
            }
        }
        if self.updater.modal_open {
            layers.push(update_modal::view_modal(&self.updater).map(Message::Update));
        }
        if self.palette.open {
            layers.push(crate::ui::palette::view(&self.palette).map(Message::Palette));
        }
        if layers.len() == 1 {
            layers.pop().expect("non-empty")
        } else {
            stack(layers).width(Length::Fill).height(Length::Fill).into()
        }
    }

    fn trigger_manual_update_check(&mut self) -> Task<Message> {
        if matches!(
            self.updater.stage,
            UpdateStage::Checking | UpdateStage::Downloading | UpdateStage::Installing
        ) {
            return Task::none();
        }
        self.updater.stage = UpdateStage::Checking;
        self.updater.error = None;
        Task::perform(
            async move {
                fetch_latest_release("TheLunatic1/Glyph", None)
                    .await
                    .map_err(|e| e.to_string())
            },
            |result| Message::DesktopUpdateChecked {
                silent: false,
                result,
            },
        )
    }

    fn handle_update_msg(&mut self, msg: UpdateMsg) -> Task<Message> {
        match msg {
            UpdateMsg::OpenModal => {
                self.updater.modal_open = true;
                Task::none()
            }
            UpdateMsg::CloseModal => {
                if self.updater.stage != UpdateStage::Installing {
                    self.updater.modal_open = false;
                }
                Task::none()
            }
            UpdateMsg::DismissBanner => {
                self.updater.banner_dismissed = true;
                Task::none()
            }
            UpdateMsg::StartDownload => self.start_update_download(),
            UpdateMsg::CancelDownload => {
                if let Some(tok) = self.update_cancel.take() {
                    tok.cancel();
                }
                self.updater.progress = None;
                self.updater.stage = if self.updater.info.is_some() {
                    UpdateStage::Available
                } else {
                    UpdateStage::Idle
                };
                Task::none()
            }
            UpdateMsg::InstallNow => self.install_downloaded_update(),
            UpdateMsg::OpenReleasePage => {
                let url = self
                    .updater
                    .info
                    .as_ref()
                    .map(|r| r.html_url.as_str())
                    .unwrap_or("https://github.com/TheLunatic1/Glyph/releases/latest");
                titlebar::open_url_in_browser(url);
                Task::none()
            }
            UpdateMsg::CheckNow => self.trigger_manual_update_check(),
            UpdateMsg::Noop => Task::none(),
        }
    }

    fn start_update_download(&mut self) -> Task<Message> {
        if matches!(
            self.updater.stage,
            UpdateStage::Downloading | UpdateStage::Installing
        ) {
            return Task::none();
        }
        let Some(rel) = self.updater.info.clone() else {
            return self.trigger_manual_update_check();
        };
        if let Some(prev) = self.update_cancel.take() {
            prev.cancel();
        }
        let token = tokio_util::sync::CancellationToken::new();
        self.update_cancel = Some(token.clone());
        self.updater.stage = UpdateStage::Downloading;
        self.updater.modal_open = true;
        self.updater.progress = None;
        self.updater.downloaded = None;
        self.updater.error = None;

        let dest_dir = self.with_vault(|v| v.data_dir().join("updates").join(&rel.tag_name));
        tokio::spawn(async move {
            let tx = app_bus_sender();
            let res = download_update_asset(&rel, &dest_dir, token, move |prog| {
                let _ = tx.send(BusEvent::UpdateDownloadProgress(prog));
            })
            .await
            .map_err(|e| e.to_string());
            let _ = app_bus_sender().send(BusEvent::UpdateDownloadFinished(res));
        });
        Task::none()
    }

    fn install_downloaded_update(&mut self) -> Task<Message> {
        let Some(dl) = self.updater.downloaded.clone() else {
            return Task::none();
        };
        match launch_update_installer(&dl) {
            Ok(()) => {
                if dl.install_kind == UpdateInstallKind::MacOsDmgOpen {
                    self.updater.modal_open = false;
                    self.updater.stage = UpdateStage::Available;
                    Task::none()
                } else {
                    self.updater.stage = UpdateStage::Installing;
                    self.quit_all()
                }
            }
            Err(e) => {
                self.updater.stage = UpdateStage::Error;
                self.updater.error = Some(e.to_string());
                self.updater.modal_open = true;
                Task::none()
            }
        }
    }

    fn open_mobile_modal(&mut self) -> Task<Message> {
        self.modal = Some(ManagerModal::Mobile(MobileModalState::default()));
        Task::perform(
            async {
                fetch_latest_release("TheLunatic1/glyph-app", Some(".apk"))
                    .await
                    .map_err(|e| e.to_string())
            },
            Message::MobileReleaseFetched,
        )
    }

    fn handle_api_ui_event(&mut self, ev: ApiUiEvent) -> Task<Message> {
        match ev {
            ApiUiEvent::SessionConnected {
                server_id,
                os,
                session,
                ..
            } => {
                if os.is_some() {
                    self.with_vault(|v| {
                        let _ = v.update_server_os(&server_id, os);
                    });
                    self.refresh_vault_cache();
                }

                if let Some((&existing_wid, _)) = self
                    .server_windows
                    .iter()
                    .find(|(_, s)| s.server.id == server_id)
                {
                    return window::gain_focus(existing_wid);
                }

                let Some(server) = self.with_vault(|v| v.get_server(&server_id)) else {
                    return Task::none();
                };
                let (cached_commands, cached_secrets) =
                    self.with_vault(|v| (v.list_commands(&server.id), v.list_secrets(&server.id)));

                let (new_wid, open_task) = window::open(server_window_settings());
                let (ev_tx, mut ev_rx) = mpsc::unbounded_channel::<SessionEvent>();
                let bus = app_bus_sender();
                tokio::spawn(async move {
                    while let Some(sev) = ev_rx.recv().await {
                        let _ = bus.send(BusEvent::Session(new_wid, sev));
                    }
                });
                session.start_stat_polling(ev_tx);
                terminal_page::spawn_open_shell(
                    session.clone(),
                    "tab-main".to_string(),
                    100,
                    30,
                    new_wid,
                );

        self.server_windows.insert(
                    new_wid,
                    ServerWindowState::new(server, session, cached_commands, cached_secrets),
                );
                self.init_server_termius_hooks(new_wid);
                self.rebuild_palette();
                Task::batch([open_task.discard(), window::gain_focus(new_wid)])
            }
            ApiUiEvent::SwitchTab { server_id, tab } => {
                let ui_tab = match tab {
                    ApiUiTab::Dashboard => ServerTab::Dashboard,
                    ApiUiTab::Terminal => ServerTab::Terminal,
                    ApiUiTab::Sftp => ServerTab::Sftp,
                    ApiUiTab::Commands => ServerTab::Commands,
                    ApiUiTab::Secrets => ServerTab::Secrets,
                    ApiUiTab::Tunnels => ServerTab::Tunnels,
                    ApiUiTab::Containers => ServerTab::Containers,
                    ApiUiTab::Vnc => ServerTab::Vnc,
                };
                let mut focus_wid = None;
                for (&wid, srv) in &mut self.server_windows {
                    if srv.server.id == server_id {
                        srv.active_tab = ui_tab;
                        focus_wid = Some(wid);
                        match ui_tab {
                            ServerTab::Tunnels => spawn_list_tunnels(wid, srv.session.clone()),
                            ServerTab::Containers => {
                                srv.containers.loading = true;
                                spawn_list_containers(wid, srv.session.clone());
                            }
                            ServerTab::Sftp if !srv.sftp.opened => {
                                srv.sftp.loading = true;
                                spawn_sftp_readdir(
                                    wid,
                                    srv.session.clone(),
                                    srv.sftp.current_path.clone(),
                                );
                            }
                            _ => {}
                        }
                    }
                }
                if let Some(wid) = focus_wid {
                    window::gain_focus(wid)
                } else {
                    Task::none()
                }
            }
            ApiUiEvent::AgentCommandStarted { server_id, command } => {
                let mut focus_wid = None;
                for (&wid, srv) in &mut self.server_windows {
                    if srv.server.id == server_id {
                        srv.active_tab = ServerTab::Terminal;
                        srv.terminal.append_agent_command(&command);
                        focus_wid = Some(wid);
                    }
                }
                if let Some(wid) = focus_wid {
                    window::gain_focus(wid)
                } else {
                    Task::none()
                }
            }
            ApiUiEvent::AgentCommandOutput { server_id, output } => {
                for srv in self.server_windows.values_mut() {
                    if srv.server.id == server_id {
                        srv.active_tab = ServerTab::Terminal;
                        srv.terminal.append_agent_output(&output);
                    }
                }
                Task::none()
            }
            ApiUiEvent::AgentSftpFile {
                server_id,
                action,
                path,
                content,
            } => {
                let parent_dir = match path.rsplit_once('/') {
                    Some(("", _)) | None => "/".to_string(),
                    Some((dir, _)) => dir.to_string(),
                };
                let mut focus_wid = None;
                for (&wid, srv) in &mut self.server_windows {
                    if srv.server.id == server_id {
                        srv.active_tab = ServerTab::Sftp;
                        srv.sftp.opened = true;
                        srv.sftp.loading = true;
                        srv.sftp.error = None;
                        spawn_sftp_readdir(wid, srv.session.clone(), parent_dir.clone());
                        srv.sftp.editor = Some(SftpEditorState {
                            path: path.clone(),
                            content: text_editor::Content::with_text(&content),
                            dirty: false,
                            saving: false,
                        });
                        let verb = if action == "write" { "Wrote" } else { "Read" };
                        srv.sftp.status = Some(format!("[MCP Agent] {verb} {path}"));
                        focus_wid = Some(wid);
                    }
                }
                if let Some(wid) = focus_wid {
                    window::gain_focus(wid)
                } else {
                    Task::none()
                }
            }
            ApiUiEvent::SessionDisconnected { server_id } => {
                let target_wids: Vec<window::Id> = self
                    .server_windows
                    .iter()
                    .filter(|(_, s)| s.server.id == server_id)
                    .map(|(&wid, _)| wid)
                    .collect();
                let mut tasks = Vec::with_capacity(target_wids.len());
                for wid in target_wids {
                    if let Some(mut srv) = self.server_windows.remove(&wid) {
                        srv.closing = true;
                        if let Some(vh) = &srv.vnc.handle {
                            vh.disconnect();
                        }
                        tasks.push(window::close(wid));
                    }
                }
                self.rebuild_palette();
                Task::batch(tasks)
            }
            ApiUiEvent::AgentTelemetryUpdated {
                server_id,
                snapshot,
            } => {
                let mut focus_wid = None;
                for (&wid, srv) in &mut self.server_windows {
                    if srv.server.id == server_id {
                        srv.active_tab = ServerTab::Dashboard;
                        srv.dashboard.stats = *snapshot.clone();
                        focus_wid = Some(wid);
                    }
                }
                if let Some(wid) = focus_wid {
                    window::gain_focus(wid)
                } else {
                    Task::none()
                }
            }
            ApiUiEvent::AgentSftpDir {
                server_id,
                path,
                entries,
                status,
            } => {
                let mut focus_wid = None;
                for (&wid, srv) in &mut self.server_windows {
                    if srv.server.id == server_id {
                        srv.active_tab = ServerTab::Sftp;
                        srv.sftp.opened = true;
                        srv.sftp.loading = false;
                        srv.sftp.error = None;
                        srv.sftp.current_path = path.clone();
                        srv.sftp.entries = entries.clone();
                        srv.sftp.editor = None;
                        srv.sftp.status = Some(status.clone());
                        focus_wid = Some(wid);
                    }
                }
                if let Some(wid) = focus_wid {
                    window::gain_focus(wid)
                } else {
                    Task::none()
                }
            }
            ApiUiEvent::AgentSftpRefresh {
                server_id,
                dir,
                status,
            } => {
                let mut focus_wid = None;
                for (&wid, srv) in &mut self.server_windows {
                    if srv.server.id == server_id {
                        srv.active_tab = ServerTab::Sftp;
                        srv.sftp.opened = true;
                        srv.sftp.loading = true;
                        srv.sftp.error = None;
                        srv.sftp.editor = None;
                        srv.sftp.status = Some(status.clone());
                        spawn_sftp_readdir(wid, srv.session.clone(), dir.clone());
                        focus_wid = Some(wid);
                    }
                }
                if let Some(wid) = focus_wid {
                    window::gain_focus(wid)
                } else {
                    Task::none()
                }
            }
            ApiUiEvent::AgentTunnelDirection {
                server_id,
                direction,
            } => {
                let dir_enum = match direction.as_str() {
                    "remote" => UiTunnelDirection::Remote,
                    "dynamic" => UiTunnelDirection::Dynamic,
                    _ => UiTunnelDirection::Local,
                };
                let mut focus_wid = None;
                for (&wid, srv) in &mut self.server_windows {
                    if srv.server.id == server_id {
                        srv.active_tab = ServerTab::Tunnels;
                        srv.tunnels.direction = dir_enum;
                        spawn_list_tunnels(wid, srv.session.clone());
                        focus_wid = Some(wid);
                    }
                }
                if let Some(wid) = focus_wid {
                    window::gain_focus(wid)
                } else {
                    Task::none()
                }
            }
            ApiUiEvent::AgentContainerDetail {
                server_id,
                container_id,
                show_inspect,
                logs,
                inspect,
            } => {
                let mut focus_wid = None;
                for (&wid, srv) in &mut self.server_windows {
                    if srv.server.id == server_id {
                        srv.active_tab = ServerTab::Containers;
                        if srv.containers.containers.is_empty() {
                            spawn_list_containers(wid, srv.session.clone());
                        }
                        let matched = srv
                            .containers
                            .containers
                            .iter()
                            .find(|c| {
                                c.id == container_id
                                    || c.id.starts_with(&container_id)
                                    || c.name == container_id
                            })
                            .cloned();
                        let summary = matched.unwrap_or_else(|| ContainerSummary {
                            id: container_id.clone(),
                            image: String::new(),
                            name: container_id.clone(),
                            status: String::new(),
                            state: "running".to_string(),
                        });
                        let resolved_id = summary.id.clone();
                        if !srv.containers.open_tabs.contains(&resolved_id) {
                            srv.containers.open_tabs.push(resolved_id.clone());
                        }
                        let det = srv
                            .containers
                            .details
                            .entry(resolved_id.clone())
                            .or_insert_with(|| ContainerTabDetail::new(summary));
                        det.subtab = if show_inspect {
                            ContainerSubTab::Inspect
                        } else {
                            ContainerSubTab::Logs
                        };
                        det.loading = false;
                        if let Some(ref l) = logs {
                            det.logs = l.clone();
                        }
                        if let Some(ref insp) = inspect {
                            det.inspect = Some((**insp).clone());
                        }
                        srv.containers.active_tab = Some(resolved_id);
                        focus_wid = Some(wid);
                    }
                }
                if let Some(wid) = focus_wid {
                    window::gain_focus(wid)
                } else {
                    Task::none()
                }
            }
            ApiUiEvent::AgentComposeResult {
                server_id,
                dir,
                output,
            } => {
                let mut focus_wid = None;
                for (&wid, srv) in &mut self.server_windows {
                    if srv.server.id == server_id {
                        srv.active_tab = ServerTab::Containers;
                        srv.containers.active_tab = None;
                        srv.containers.show_compose = true;
                        srv.containers.compose_path = dir.clone();
                        srv.containers.compose_busy = false;
                        srv.containers.compose_output = Some(output.clone());
                        spawn_list_containers(wid, srv.session.clone());
                        focus_wid = Some(wid);
                    }
                }
                if let Some(wid) = focus_wid {
                    window::gain_focus(wid)
                } else {
                    Task::none()
                }
            }
            ApiUiEvent::AgentOpenVnc {
                server_id,
                host,
                port,
                auto_connect,
            } => {
                let mut focus_wid = None;
                for (&wid, srv) in &mut self.server_windows {
                    if srv.server.id == server_id {
                        srv.active_tab = ServerTab::Vnc;
                        srv.vnc.host = host.clone();
                        srv.vnc.port = port.to_string();
                        srv.vnc.error = None;
                        if auto_connect && !srv.vnc.connected && !srv.vnc.connecting {
                            srv.vnc.connecting = true;
                            spawn_vnc_connect(wid, srv.session.clone(), host.clone(), port, None);
                        }
                        focus_wid = Some(wid);
                    }
                }
                if let Some(wid) = focus_wid {
                    window::gain_focus(wid)
                } else {
                    Task::none()
                }
            }
            ApiUiEvent::SecretsChanged { server_id } => {
                let updated = self.with_vault(|v| v.list_secrets(&server_id));
                for srv in self.server_windows.values_mut() {
                    if srv.server.id == server_id {
                        srv.cached_secrets = updated.clone();
                    }
                }
                Task::none()
            }
            ApiUiEvent::CommandsChanged { server_id } => {
                let updated = self.with_vault(|v| v.list_commands(&server_id));
                for srv in self.server_windows.values_mut() {
                    if srv.server.id == server_id {
                        srv.cached_commands = updated.clone();
                    }
                }
                Task::none()
            }
            ApiUiEvent::TunnelsChanged { server_id } => {
                for (&wid, srv) in &self.server_windows {
                    if srv.server.id == server_id {
                        spawn_list_tunnels(wid, srv.session.clone());
                    }
                }
                Task::none()
            }
            ApiUiEvent::VaultChanged => {
                self.refresh_vault_cache();
                self.reload_termius_caches();
                self.rebuild_palette();
                Task::none()
            }
        }
    }

    fn handle_sftp_msg(&mut self, wid: window::Id, msg: SftpPageMsg) -> Task<Message> {
        if self.handle_sftp_termius_msg(wid, msg.clone()) {
            return Task::none();
        }
        let Some(srv) = self.server_windows.get_mut(&wid) else {
            return Task::none();
        };
        let sess = srv.session.clone();

        match msg {
            SftpPageMsg::OpenRoot => {
                srv.sftp.loading = true;
                srv.sftp.error = None;
                spawn_sftp_readdir(wid, sess, "/".to_string());
            }
            SftpPageMsg::Navigate(path) => {
                srv.sftp.loading = true;
                srv.sftp.error = None;
                srv.sftp.copied_path = false;
                spawn_sftp_readdir(wid, sess, path);
            }
            SftpPageMsg::GoUp => {
                let parent = remote_parent(&srv.sftp.current_path);
                srv.sftp.loading = true;
                srv.sftp.error = None;
                spawn_sftp_readdir(wid, sess, parent);
            }
            SftpPageMsg::Refresh => {
                srv.sftp.loading = true;
                srv.sftp.error = None;
                spawn_sftp_readdir(wid, sess, srv.sftp.current_path.clone());
            }
            SftpPageMsg::CopyCurrentPath => {
                clipboard_set(srv.sftp.current_path.clone());
                srv.sftp.copied_path = true;
            }
            SftpPageMsg::ToggleSort(col) => {
                if srv.sftp.sort_col == col {
                    srv.sftp.sort_asc = !srv.sftp.sort_asc;
                } else {
                    srv.sftp.sort_col = col;
                    srv.sftp.sort_asc = true;
                }
            }
            SftpPageMsg::OpenEntry(path, is_dir) => {
                if is_dir {
                    srv.sftp.loading = true;
                    srv.sftp.error = None;
                    spawn_sftp_readdir(wid, sess, path);
                } else {
                    tokio::spawn(async move {
                        let res = sess.sftp_read_text(&path).await.map_err(|e| e.to_string());
                        let _ = app_bus_sender().send(BusEvent::SftpFileRead(wid, path, res));
                    });
                }
            }
            SftpPageMsg::DownloadFile(remote_path, name) => {
                return Task::perform(
                    async move {
                        let opt = rfd::AsyncFileDialog::new()
                            .set_title("Save Remote File")
                            .set_file_name(&name)
                            .save_file()
                            .await
                            .map(|h| h.path().to_path_buf());
                        (remote_path, opt)
                    },
                    move |(rp, opt)| Message::SftpDownloadSavePicked(wid, rp, opt),
                );
            }
            SftpPageMsg::UploadPickFiles => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Select Files to Upload via SFTP")
                            .pick_files()
                            .await
                            .unwrap_or_default()
                            .into_iter()
                            .map(|h| h.path().to_path_buf())
                            .collect()
                    },
                    move |paths| Message::SftpUploadPicked(wid, paths),
                );
            }
            SftpPageMsg::CopyItem(entry) => {
                srv.sftp.clipboard = Some(sftp_page::SftpClipboard {
                    path: entry.path,
                    name: entry.filename.clone(),
                    is_copy: true,
                });
                srv.sftp.status = Some(format!("Copied \"{}\" to SFTP clipboard", entry.filename));
            }
            SftpPageMsg::CutItem(entry) => {
                srv.sftp.clipboard = Some(sftp_page::SftpClipboard {
                    path: entry.path,
                    name: entry.filename.clone(),
                    is_copy: false,
                });
                srv.sftp.status = Some(format!("Cut \"{}\" — navigate & click Paste", entry.filename));
            }
            SftpPageMsg::PasteClipboard => {
                if let Some(clip) = srv.sftp.clipboard.take() {
                    let dst = join_remote_path(&srv.sftp.current_path, &clip.name);
                    if clip.is_copy {
                        srv.sftp.clipboard = Some(clip.clone());
                        tokio::spawn(async move {
                            let res = sess
                                .sftp_copy(&clip.path, &dst)
                                .await
                                .map(|()| format!("Copied \"{}\" to {dst}", clip.name))
                                .map_err(|e| e.to_string());
                            let _ = app_bus_sender().send(BusEvent::SftpActionDone(wid, res));
                        });
                    } else {
                        tokio::spawn(async move {
                            let res = sess
                                .sftp_rename(&clip.path, &dst)
                                .await
                                .map(|()| format!("Moved \"{}\" to {dst}", clip.name))
                                .map_err(|e| e.to_string());
                            let _ = app_bus_sender().send(BusEvent::SftpActionDone(wid, res));
                        });
                    }
                }
            }
            SftpPageMsg::OpenNewFolderPrompt => {
                srv.sftp.prompt = Some(SftpPrompt::NewFolder);
                srv.sftp.prompt_input.clear();
            }
            SftpPageMsg::OpenNewFilePrompt => {
                srv.sftp.prompt = Some(SftpPrompt::NewFile);
                srv.sftp.prompt_input.clear();
            }
            SftpPageMsg::OpenRenamePrompt(path, name) => {
                srv.sftp.prompt = Some(SftpPrompt::Rename { old_path: path });
                srv.sftp.prompt_input = name;
            }
            SftpPageMsg::OpenChmodPrompt(path, perms) => {
                srv.sftp.prompt = Some(SftpPrompt::Chmod { path });
                srv.sftp.prompt_input = format!("{:03o}", perms & 0o777);
            }
            SftpPageMsg::OpenDeletePrompt(path, name, is_dir) => {
                srv.sftp.prompt = Some(SftpPrompt::ConfirmDelete { path, name, is_dir });
                srv.sftp.prompt_input.clear();
            }
            SftpPageMsg::PromptInputChanged(v) => srv.sftp.prompt_input = v,
            SftpPageMsg::PromptCancel => {
                srv.sftp.prompt = None;
                srv.sftp.prompt_input.clear();
            }
            SftpPageMsg::PromptSubmit => {
                let Some(prompt) = srv.sftp.prompt.take() else {
                    return Task::none();
                };
                let val = srv.sftp.prompt_input.trim().to_string();
                let dir = srv.sftp.current_path.clone();
                srv.sftp.prompt_input.clear();
                match prompt {
                    SftpPrompt::NewFolder => {
                        if !val.is_empty() {
                            let target = join_remote_path(&dir, &val);
                            tokio::spawn(async move {
                                let res = sess
                                    .sftp_mkdir(&target)
                                    .await
                                    .map(|()| format!("Created folder {val}"))
                                    .map_err(|e| e.to_string());
                                let _ = app_bus_sender().send(BusEvent::SftpActionDone(wid, res));
                            });
                        }
                    }
                    SftpPrompt::NewFile => {
                        if !val.is_empty() {
                            let target = join_remote_path(&dir, &val);
                            tokio::spawn(async move {
                                let res = sess
                                    .sftp_create_empty_file(&target)
                                    .await
                                    .map(|()| format!("Created file {val}"))
                                    .map_err(|e| e.to_string());
                                let _ = app_bus_sender().send(BusEvent::SftpActionDone(wid, res));
                            });
                        }
                    }
                    SftpPrompt::Rename { old_path } => {
                        if !val.is_empty() {
                            let new_path = join_remote_path(&remote_parent(&old_path), &val);
                            tokio::spawn(async move {
                                let res = sess
                                    .sftp_rename(&old_path, &new_path)
                                    .await
                                    .map(|()| format!("Renamed to {val}"))
                                    .map_err(|e| e.to_string());
                                let _ = app_bus_sender().send(BusEvent::SftpActionDone(wid, res));
                            });
                        }
                    }
                    SftpPrompt::Chmod { path } => {
                        if let Ok(mode) = u32::from_str_radix(&val, 8) {
                            tokio::spawn(async move {
                                let res = sess
                                    .sftp_chmod(&path, mode)
                                    .await
                                    .map(|()| format!("Updated permissions on {path} to {mode:o}"))
                                    .map_err(|e| e.to_string());
                                let _ = app_bus_sender().send(BusEvent::SftpActionDone(wid, res));
                            });
                        } else {
                            srv.sftp.error = Some(format!("Invalid octal mode \"{val}\" (use e.g. 755 or 644)"));
                        }
                    }
                    SftpPrompt::ConfirmDelete { path, name, is_dir } => {
                        tokio::spawn(async move {
                            let res = sess
                                .sftp_delete(&path, is_dir)
                                .await
                                .map(|()| format!("Deleted {name}"))
                                .map_err(|e| e.to_string());
                            let _ = app_bus_sender().send(BusEvent::SftpActionDone(wid, res));
                        });
                    }
                }
            }
            SftpPageMsg::EditorAction(action) => {
                if let Some(ed) = &mut srv.sftp.editor {
                    let is_edit = matches!(action, text_editor::Action::Edit(_));
                    ed.content.perform(action);
                    if is_edit {
                        ed.dirty = true;
                    }
                }
            }
            SftpPageMsg::EditorSave => {
                if let Some(ed) = &mut srv.sftp.editor {
                    ed.saving = true;
                    let path = ed.path.clone();
                    let text_body = ed.content.text();
                    tokio::spawn(async move {
                        let res = sess
                            .sftp_write_text(&path, &text_body)
                            .await
                            .map_err(|e| e.to_string());
                        let _ = app_bus_sender().send(BusEvent::SftpFileSaved(wid, path, res));
                    });
                }
            }
            SftpPageMsg::EditorClose => {
                srv.sftp.editor = None;
                srv.sftp.status = None;
            }
            SftpPageMsg::DismissTransfer => {
                srv.sftp.transfer = None;
            }
            _ => {}
        }
        Task::none()
    }

    fn handle_commands_msg(&mut self, wid: window::Id, msg: CommandsPageMsg) -> Task<Message> {
        if self.handle_commands_termius_msg(wid, msg.clone()) {
            return Task::none();
        }
        let Some(srv) = self.server_windows.get_mut(&wid) else {
            return Task::none();
        };
        let sid = srv.server.id.clone();

        match msg {
            CommandsPageMsg::NameInput(v) => srv.commands.new_name = v,
            CommandsPageMsg::CmdInput(v) => srv.commands.new_cmd = v,
            CommandsPageMsg::AddSubmit => {
                let name = srv.commands.new_name.trim().to_string();
                let cmd = srv.commands.new_cmd.trim().to_string();
                if name.is_empty() || cmd.is_empty() {
                    srv.commands.error = Some("Enter both a Snippet Name and Shell Command.".into());
                    return Task::none();
                }
                let res = self.vault.lock().map_err(|_| anyhow::anyhow!("Vault busy")).and_then(|mut v| {
                    v.add_command(&sid, &name, &cmd)?;
                    Ok(v.list_commands(&sid))
                });
                match res {
                    Ok(list) => {
                        srv.cached_commands = list;
                        srv.commands.new_name.clear();
                        srv.commands.new_cmd.clear();
                        srv.commands.error = None;
                    }
                    Err(e) => srv.commands.error = Some(e.to_string()),
                }
            }
            CommandsPageMsg::Delete(id) => {
                if let Ok(mut v) = self.vault.lock() {
                    let _ = v.remove_command(&sid, id);
                    srv.cached_commands = v.list_commands(&sid);
                }
            }
            _ => {}
        }
        Task::none()
    }

    fn handle_secrets_msg(&mut self, wid: window::Id, msg: SecretsPageMsg) -> Task<Message> {
        let Some(srv) = self.server_windows.get_mut(&wid) else {
            return Task::none();
        };
        let sid = srv.server.id.clone();

        match msg {
            SecretsPageMsg::NameInput(v) => srv.secrets.new_name = v,
            SecretsPageMsg::ValueInput(v) => srv.secrets.new_value = v,
            SecretsPageMsg::ToggleShowValue => srv.secrets.show_value = !srv.secrets.show_value,
            SecretsPageMsg::AddSubmit => {
                let name = srv.secrets.new_name.trim().to_string();
                let val = srv.secrets.new_value.clone();
                if name.is_empty() || val.is_empty() {
                    srv.secrets.error = Some("Both Secret Alias and Secret Value are required.".into());
                    return Task::none();
                }
                let res = self.vault.lock().map_err(|_| anyhow::anyhow!("Vault busy")).and_then(|mut v| {
                    v.add_secret(&sid, &name, &val)?;
                    Ok(v.list_secrets(&sid))
                });
                match res {
                    Ok(list) => {
                        srv.cached_secrets = list;
                        srv.secrets.new_name.clear();
                        srv.secrets.new_value.clear();
                        srv.secrets.error = None;
                        srv.secrets.status = Some(format!("Stored encrypted secret \"{name}\"."));
                    }
                    Err(e) => srv.secrets.error = Some(e.to_string()),
                }
            }
            SecretsPageMsg::CopyValue(secret_id) => {
                if let Ok(Ok(Some(plain))) = self.vault.lock().map(|v| v.get_secret_value(&secret_id)) {
                    clipboard_set(plain);
                    srv.secrets.copied_id = Some(secret_id);
                    srv.secrets.status = Some("Decrypted secret copied to clipboard.".into());
                }
            }
            SecretsPageMsg::Delete(secret_id) => {
                if let Ok(mut v) = self.vault.lock() {
                    let _ = v.delete_secret(&secret_id);
                    srv.cached_secrets = v.list_secrets(&sid);
                }
            }
            SecretsPageMsg::OpenExportModal => {
                srv.secrets.bundle_modal = Some(SecretsBundleMode::Export);
                srv.secrets.bundle_password.clear();
                srv.secrets.error = None;
            }
            SecretsPageMsg::OpenImportModal => {
                srv.secrets.bundle_modal = Some(SecretsBundleMode::Import);
                srv.secrets.bundle_password.clear();
                srv.secrets.error = None;
            }
            SecretsPageMsg::CloseBundleModal => {
                srv.secrets.bundle_modal = None;
                srv.secrets.bundle_password.clear();
            }
            SecretsPageMsg::BundlePasswordInput(pw) => srv.secrets.bundle_password = pw,
            SecretsPageMsg::ToggleBundlePassword => {
                srv.secrets.show_bundle_password = !srv.secrets.show_bundle_password;
            }
            SecretsPageMsg::BundleSubmit => {
                let Some(mode) = srv.secrets.bundle_modal else {
                    return Task::none();
                };
                let pw = srv.secrets.bundle_password.clone();
                if pw.is_empty() {
                    srv.secrets.error = Some("Master password is required.".into());
                    return Task::none();
                }
                match mode {
                    SecretsBundleMode::Export => {
                        let enc_res = self
                            .vault
                            .lock()
                            .map_err(|_| anyhow::anyhow!("Vault busy"))
                            .and_then(|v| v.export_secrets_encrypted(&sid, &pw));
                        match enc_res {
                            Ok(payload) => {
                                return Task::perform(
                                    async move {
                                        let Some(h) = rfd::AsyncFileDialog::new()
                                            .set_title("Export Encrypted Secrets")
                                            .add_filter("Glyph Secrets", &["glyph"])
                                            .set_file_name("glyph-secrets.glyph")
                                            .save_file()
                                            .await
                                        else {
                                            return Ok(None);
                                        };
                                        let p = h.path().to_path_buf();
                                        tokio::fs::write(&p, payload.as_bytes())
                                            .await
                                            .map_err(|e| format!("Write failed: {e}"))?;
                                        Ok(Some(p.display().to_string()))
                                    },
                                    move |r| Message::SecretsExportDone(wid, r),
                                );
                            }
                            Err(e) => srv.secrets.error = Some(e.to_string()),
                        }
                    }
                    SecretsBundleMode::Import => {
                        return Task::perform(
                            async move {
                                let Some(h) = rfd::AsyncFileDialog::new()
                                    .set_title("Import Encrypted Secrets (.glyph)")
                                    .add_filter("Glyph Secrets", &["glyph"])
                                    .pick_file()
                                    .await
                                else {
                                    return Ok(None);
                                };
                                let txt = tokio::fs::read_to_string(h.path())
                                    .await
                                    .map_err(|e| format!("Read failed: {e}"))?;
                                Ok(Some(txt))
                            },
                            move |r| Message::SecretsImportLoaded(wid, r),
                        );
                    }
                }
            }
        }
        Task::none()
    }

    fn handle_tunnels_msg(&mut self, wid: window::Id, msg: TunnelsPageMsg) -> Task<Message> {
        if self.handle_tunnels_termius_msg(wid, msg.clone()) {
            return Task::none();
        }
        let Some(srv) = self.server_windows.get_mut(&wid) else {
            return Task::none();
        };
        let sess = srv.session.clone();

        match msg {
            TunnelsPageMsg::SelectDirection(d) => srv.tunnels.direction = d,
            TunnelsPageMsg::BindPortChanged(v) => srv.tunnels.bind_port = v,
            TunnelsPageMsg::TargetHostChanged(v) => srv.tunnels.target_host = v,
            TunnelsPageMsg::TargetPortChanged(v) => srv.tunnels.target_port = v,
            TunnelsPageMsg::ProtocolSelected(p) => srv.tunnels.protocol = p,
            TunnelsPageMsg::ToggleScheme(tid) => {
                let entry = srv.tunnels.schemes.entry(tid).or_insert(false);
                *entry = !*entry;
            }
            TunnelsPageMsg::OpenUrl(url) => titlebar::open_url_in_browser(&url),
            TunnelsPageMsg::Refresh => spawn_list_tunnels(wid, sess),
            TunnelsPageMsg::StopTunnel(tid) => {
                srv.tunnels.busy = true;
                tokio::spawn(async move {
                    let res = match sess.stop_tunnel_by_id(&tid).await {
                        Ok(()) => Ok(sess.list_tunnels().await),
                        Err(e) => Err(e.to_string()),
                    };
                    let _ = app_bus_sender().send(BusEvent::TunnelOpDone(wid, res));
                });
            }
            _ => {}
        }
        Task::none()
    }

    fn handle_containers_msg(&mut self, wid: window::Id, msg: ContainersPageMsg) -> Task<Message> {
        let Some(srv) = self.server_windows.get_mut(&wid) else {
            return Task::none();
        };
        let sess = srv.session.clone();

        match msg {
            ContainersPageMsg::SelectListTab => srv.containers.active_tab = None,
            ContainersPageMsg::SwitchContainerTab(cid) => {
                srv.containers.active_tab = Some(cid.clone());
                if let Some(det) = srv.containers.details.get(&cid) {
                    let running = det.summary.state.eq_ignore_ascii_case("running");
                    spawn_load_container_detail(wid, sess, cid, running);
                }
            }
            ContainersPageMsg::CloseContainerTab(cid) => {
                srv.containers.open_tabs.retain(|t| t != &cid);
                srv.containers.details.remove(&cid);
                if srv.containers.active_tab.as_deref() == Some(cid.as_str()) {
                    srv.containers.active_tab = srv.containers.open_tabs.last().cloned();
                }
            }
            ContainersPageMsg::OpenContainerTab(summary) => {
                let cid = summary.id.clone();
                let running = summary.state.eq_ignore_ascii_case("running");
                if !srv.containers.open_tabs.contains(&cid) {
                    srv.containers.open_tabs.push(cid.clone());
                }
                srv.containers
                    .details
                    .entry(cid.clone())
                    .or_insert_with(|| ContainerTabDetail::new(summary));
                srv.containers.active_tab = Some(cid.clone());
                srv.last_container_poll = Instant::now();
                spawn_load_container_detail(wid, sess, cid, running);
            }
            ContainersPageMsg::Refresh => {
                srv.containers.loading = true;
                spawn_list_containers(wid, sess);
            }
            ContainersPageMsg::SearchChanged(q) => srv.containers.search = q,
            ContainersPageMsg::ToggleCompose => {
                srv.containers.show_compose = !srv.containers.show_compose;
            }
            ContainersPageMsg::ComposePathChanged(p) => srv.containers.compose_path = p,
            ContainersPageMsg::ComposeUp => {
                let dir = srv.containers.compose_path.clone();
                srv.containers.compose_busy = true;
                tokio::spawn(async move {
                    let res = sess.compose_action(&dir, true).await.map_err(|e| e.to_string());
                    let _ = app_bus_sender().send(BusEvent::ComposeDone(wid, res));
                });
            }
            ContainersPageMsg::ComposeDown => {
                let dir = srv.containers.compose_path.clone();
                srv.containers.compose_busy = true;
                tokio::spawn(async move {
                    let res = sess.compose_action(&dir, false).await.map_err(|e| e.to_string());
                    let _ = app_bus_sender().send(BusEvent::ComposeDone(wid, res));
                });
            }
            ContainersPageMsg::Action(cid, action) => {
                tokio::spawn(async move {
                    let res = sess
                        .container_action(&cid, action)
                        .await
                        .map_err(|e| e.to_string());
                    let _ = app_bus_sender().send(BusEvent::ContainerActionDone(wid, res));
                });
            }
            ContainersPageMsg::SelectSubTab(cid, subtab) => {
                if let Some(det) = srv.containers.details.get_mut(&cid) {
                    det.subtab = subtab;
                    let running = det.summary.state.eq_ignore_ascii_case("running");
                    spawn_load_container_detail(wid, sess, cid, running);
                }
            }
            ContainersPageMsg::RefreshDetail(cid) => {
                if let Some(det) = srv.containers.details.get(&cid) {
                    let running = det.summary.state.eq_ignore_ascii_case("running");
                    spawn_load_container_detail(wid, sess, cid, running);
                }
            }
            ContainersPageMsg::RestartPolicyChanged(cid, val) => {
                if let Some(det) = srv.containers.details.get_mut(&cid) {
                    det.restart_policy = val;
                }
            }
            ContainersPageMsg::MemLimitChanged(cid, val) => {
                if let Some(det) = srv.containers.details.get_mut(&cid) {
                    det.mem_limit = val;
                }
            }
            ContainersPageMsg::CpuLimitChanged(cid, val) => {
                if let Some(det) = srv.containers.details.get_mut(&cid) {
                    det.cpu_limit = val;
                }
            }
            ContainersPageMsg::ApplyUpdate(cid) => {
                if let Some(det) = srv.containers.details.get(&cid) {
                    let r = Some(det.restart_policy.clone()).filter(|s| !s.trim().is_empty());
                    let m = Some(det.mem_limit.clone()).filter(|s| !s.trim().is_empty());
                    let c = Some(det.cpu_limit.clone()).filter(|s| !s.trim().is_empty());
                    let cid_c = cid.clone();
                    tokio::spawn(async move {
                        let res = sess
                            .container_update(&cid_c, r.as_deref(), m.as_deref(), c.as_deref())
                            .await
                            .map_err(|e| e.to_string());
                        let _ = app_bus_sender().send(BusEvent::ContainerUpdateDone(wid, cid_c, res));
                    });
                }
            }
        }
        Task::none()
    }

    fn handle_vnc_msg(&mut self, wid: window::Id, msg: VncPageMsg) -> Task<Message> {
        let Some(srv) = self.server_windows.get_mut(&wid) else {
            return Task::none();
        };

        match msg {
            VncPageMsg::HostChanged(h) => srv.vnc.host = h,
            VncPageMsg::PortChanged(p) => srv.vnc.port = p,
            VncPageMsg::PasswordChanged(pw) => srv.vnc.password = pw,
            VncPageMsg::ToggleShowPassword => srv.vnc.show_password = !srv.vnc.show_password,
            VncPageMsg::Connect => {
                let port: u16 = match srv.vnc.port.trim().parse() {
                    Ok(p) if p > 0 => p,
                    _ => {
                        srv.vnc.error = Some("Enter a valid VNC port (e.g. 5900).".into());
                        return Task::none();
                    }
                };
                let host = if srv.vnc.host.trim().is_empty() {
                    "127.0.0.1".to_string()
                } else {
                    srv.vnc.host.trim().to_string()
                };
                let pw = if srv.vnc.password.is_empty() {
                    None
                } else {
                    Some(srv.vnc.password.clone())
                };
                srv.vnc.connecting = true;
                srv.vnc.error = None;
                spawn_vnc_connect(wid, srv.session.clone(), host, port, pw);
            }
            VncPageMsg::Disconnect => {
                if let Some(vh) = srv.vnc.handle.take() {
                    vh.disconnect();
                }
                srv.vnc.connected = false;
                srv.vnc.connecting = false;
                srv.vnc.frame = None;
            }
            VncPageMsg::ToggleFit => srv.vnc.fit_to_window = !srv.vnc.fit_to_window,
            VncPageMsg::SendCtrlAltDel => {
                if let Some(vh) = &srv.vnc.handle {
                    vh.send_ctrl_alt_del();
                }
            }
            VncPageMsg::SendLocalClipboard => {
                if let (Some(vh), Some(txt)) = (&srv.vnc.handle, clipboard_get()) {
                    vh.send_clipboard(txt);
                }
            }
            VncPageMsg::CopyRemoteClipboard => {
                if let Some(txt) = &srv.vnc.server_clipboard {
                    clipboard_set(txt.clone());
                }
            }
            VncPageMsg::RefreshFrame => {
                if let Some(vh) = &srv.vnc.handle {
                    vh.request_refresh(false);
                }
            }
            VncPageMsg::Viewport(ev) => match ev {
                VncUiEvent::Pointer { x, y, mask } => {
                    if let Some(vh) = &srv.vnc.handle {
                        vh.send_pointer(x, y, mask);
                    }
                }
                VncUiEvent::Key { keysym, down } => {
                    if let Some(vh) = &srv.vnc.handle {
                        vh.send_key(keysym, down);
                    }
                }
                VncUiEvent::Focused => srv.vnc.focused = true,
            },
        }
        Task::none()
    }
}

pub(crate) fn remote_parent(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() || trimmed == "/" {
        return "/".to_string();
    }
    match trimmed.rfind('/') {
        Some(0) | None => "/".to_string(),
        Some(pos) => trimmed[..pos].to_string(),
    }
}

pub(crate) fn spawn_sftp_readdir(wid: window::Id, sess: SshSession, path: String) {
    tokio::spawn(async move {
        let res = sess.sftp_readdir(&path).await.map_err(|e| e.to_string());
        let _ = app_bus_sender().send(BusEvent::SftpListed(wid, path, res));
    });
}

pub(crate) fn spawn_sftp_readdir_peer(wid: window::Id, sess: SshSession, path: String) {
    tokio::spawn(async move {
        let res = sess.sftp_readdir(&path).await.map_err(|e| e.to_string());
        let _ = app_bus_sender().send(BusEvent::SftpPeerListed(wid, path, res));
    });
}

fn spawn_list_tunnels(wid: window::Id, sess: SshSession) {
    tokio::spawn(async move {
        let list = sess.list_tunnels().await;
        let _ = app_bus_sender().send(BusEvent::TunnelsListed(wid, list));
    });
}

fn spawn_list_containers(wid: window::Id, sess: SshSession) {
    tokio::spawn(async move {
        let res = sess
            .list_containers()
            .await
            .map(|(_, list)| list)
            .map_err(|e| e.to_string());
        let _ = app_bus_sender().send(BusEvent::ContainersListed(wid, res));
    });
}

fn spawn_load_container_detail(
    wid: window::Id,
    sess: SshSession,
    id: String,
    is_running: bool,
) {
    tokio::spawn(async move {
        let (logs_res, inspect_res) = tokio::join!(
            sess.container_logs(&id, 250),
            sess.container_inspect(&id, is_running)
        );
        let _ = app_bus_sender().send(BusEvent::ContainerDetailLoaded {
            window_id: wid,
            id,
            logs: logs_res.map_err(|e| e.to_string()),
            inspect: inspect_res.map_err(|e| e.to_string()),
        });
    });
}

fn spawn_vnc_connect(
    wid: window::Id,
    sess: SshSession,
    host: String,
    port: u16,
    password: Option<String>,
) {
    let bus = app_bus_sender();
    tokio::spawn(async move {
        match sess.open_direct_tcpip(&host, port).await {
            Ok(stream) => {
                let (ev_tx, mut ev_rx) = mpsc::unbounded_channel::<VncEvent>();
                let bus_ev = bus.clone();
                tokio::spawn(async move {
                    while let Some(vev) = ev_rx.recv().await {
                        let _ = bus_ev.send(BusEvent::VncEvent(wid, vev));
                    }
                });
                let res = connect_vnc_stream(stream, password, ev_tx)
                    .await
                    .map_err(|e| e.to_string());
                let _ = bus.send(BusEvent::VncConnected(wid, res));
            }
            Err(e) => {
                let _ = bus.send(BusEvent::VncConnected(wid, Err(e.to_string())));
            }
        }
    });
}

pub(crate) fn display_server_name(srv: &ServerSummary) -> String {
    if srv.name.trim().is_empty() {
        srv.host.clone()
    } else {
        srv.name.clone()
    }
}

fn load_window_icon() -> Option<window::Icon> {
    let img = image::load_from_memory(LOGO_PNG)
        .ok()?
        .resize(64, 64, image::imageops::FilterType::Lanczos3);
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    window::icon::from_rgba(rgba.into_raw(), w, h).ok()
}

fn manager_window_settings() -> window::Settings {
    window::Settings {
        size: Size::new(1200.0, 800.0),
        min_size: Some(Size::new(880.0, 600.0)),
        decorations: false,
        exit_on_close_request: false,
        icon: load_window_icon(),
        ..Default::default()
    }
}

pub(crate) fn server_window_settings() -> window::Settings {
    window::Settings {
        size: Size::new(1280.0, 820.0),
        min_size: Some(Size::new(960.0, 640.0)),
        decorations: false,
        exit_on_close_request: false,
        icon: load_window_icon(),
        ..Default::default()
    }
}

pub(crate) fn clipboard_set(txt: String) {
    if let Ok(mut cb) = arboard::Clipboard::new() {
        let _ = cb.set_text(txt);
    }
}

pub(crate) fn clipboard_get() -> Option<String> {
    arboard::Clipboard::new().ok().and_then(|mut cb| cb.get_text().ok())
}
