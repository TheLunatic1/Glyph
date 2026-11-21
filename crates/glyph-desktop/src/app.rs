use crate::theme::{
    dark_900_bg, dark_card, glyph_theme, pill_badge, rgba, BRAND_300, BRAND_400, BRAND_500,
    DARK_700, FONT_MONO, FONT_MONO_BOLD, JETBRAINS_MONO_BOLD, JETBRAINS_MONO_REGULAR, LOGO_PNG,
    TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY,
};
use crate::tray::{TrayCommand, TrayManager};
use crate::ui::dashboard::{self, DashboardMsg, DashboardState};
use crate::ui::icons;
use crate::ui::manager::{self, ManagerMsg, ServerFormState};
use crate::ui::modals::{
    self, ConnectingModalState, DeleteConfirmState, ExportModalState, HostKeyPromptUi,
    ImportModalState, ImportStep, ManagerModal, ModalMsg, SettingsModalState, SshKeysModalState,
};
use crate::ui::sidebar::{self, ServerTab, SidebarMsg};
use crate::ui::splash;
use crate::ui::terminal_page::{self, TerminalPageMsg, TerminalPageState};
use crate::ui::titlebar::{self, TitleBarMsg};
use glyph_core::models::{
    AuthMethod, CommandSnippet, SecretSummary, ServerEntry, ServerSummary, SshKeySummary,
};
use glyph_core::ssh::{
    HostKeyPrompt, SessionEvent, ShellEvent, ShellHandle, SshSession, TrustedHostRecord,
};
use glyph_core::vault::VaultManager;
use glyph_terminal::TerminalEvent;
use iced::futures::future::pending;
use iced::futures::{SinkExt, Stream};
use iced::widget::{column, container, row, stack, text};
use iced::{alignment, window, Border, Element, Length, Size, Subscription, Task, Theme};
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};

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

// ── Top-Level Messages & State ────────────────────────────────────────────────

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum Message {
    Tick,
    Bus(BusEvent),
    WindowCloseRequested(window::Id),
    TitleBar(window::Id, TitleBarMsg),
    Manager(ManagerMsg),
    Modal(ModalMsg),
    Sidebar(window::Id, SidebarMsg),
    Dashboard(window::Id, DashboardMsg),
    Terminal(window::Id, TerminalPageMsg),

    FormPrivateKeyPicked(Option<String>),
    ExportFinished(Result<Option<String>, String>),
    ImportFileLoaded(Result<Option<(String, String)>, String>),
    SshKeyFileLoaded(Result<Option<(String, String)>, String>),
}

struct ServerWindowState {
    server: ServerSummary,
    session: SshSession,
    active_tab: ServerTab,
    dashboard: DashboardState,
    terminal: TerminalPageState,
    cached_commands: Vec<CommandSnippet>,
    cached_secrets: Vec<SecretSummary>,
    closing: bool,
}

pub struct GlyphApp {
    vault: VaultManager,
    servers: Vec<ServerSummary>,
    ssh_keys: Vec<SshKeySummary>,
    tray: Option<TrayManager>,

    manager_window: Option<window::Id>,
    manager_visible: bool,
    show_splash: bool,
    boot_instant: Instant,
    disconnect_banner: Option<String>,
    form: Option<ServerFormState>,
    modal: Option<ManagerModal>,
    connect_started_at: Option<Instant>,

    server_windows: BTreeMap<window::Id, ServerWindowState>,
    is_quitting: bool,
}

pub fn run() -> iced::Result {
    iced::daemon(GlyphApp::boot, GlyphApp::update, GlyphApp::view)
        .title(GlyphApp::title)
        .theme(GlyphApp::theme)
        .subscription(GlyphApp::subscription)
        .font(JETBRAINS_MONO_REGULAR)
        .font(JETBRAINS_MONO_BOLD)
        .default_font(FONT_MONO)
        .antialiasing(true)
        .run()
}

impl GlyphApp {
    fn boot() -> (Self, Task<Message>) {
        let vault = VaultManager::open_default().expect("Failed to initialize Glyph encrypted Vault");
        let servers = vault.list_servers();
        let ssh_keys = vault.list_ssh_keys();
        let tray = TrayManager::init(&servers);

        let (manager_id, open_task) = window::open(manager_window_settings());

        let app = Self {
            vault,
            servers,
            ssh_keys,
            tray,
            manager_window: Some(manager_id),
            manager_visible: true,
            show_splash: true,
            boot_instant: Instant::now(),
            disconnect_banner: None,
            form: None,
            modal: None,
            connect_started_at: None,
            server_windows: BTreeMap::new(),
            is_quitting: false,
        };

        (app, open_task.discard())
    }

    fn title(&self, id: window::Id) -> String {
        if let Some(srv) = self.server_windows.get(&id) {
            let label = display_server_name(&srv.server);
            format!(
                "Glyph — {label} ({}@{})",
                srv.server.username, srv.server.host
            )
        } else {
            "Glyph".to_string()
        }
    }

    fn theme(&self, _id: window::Id) -> Theme {
        glyph_theme()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            Subscription::run(bus_stream).map(Message::Bus),
            window::close_requests().map(Message::WindowCloseRequested),
            iced::time::every(Duration::from_millis(120)).map(|_| Message::Tick),
        ])
    }

    fn refresh_vault_cache(&mut self) {
        self.servers = self.vault.list_servers();
        self.ssh_keys = self.vault.list_ssh_keys();
        if let Some(tray) = &mut self.tray {
            tray.update_servers(&self.servers);
        }
    }

    fn show_manager_window(&mut self) -> Task<Message> {
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
        self.server_windows.clear();
        for sess in sessions {
            tokio::spawn(async move {
                sess.disconnect().await;
            });
        }
        iced::exit()
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Tick => self.handle_tick(),
            Message::WindowCloseRequested(wid) => self.handle_window_close(wid),
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
                                if let Some(stem) = std::path::Path::new(&path)
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
        }
    }

    fn handle_tick(&mut self) -> Task<Message> {
        if self.show_splash && self.boot_instant.elapsed() >= Duration::from_millis(650) {
            self.show_splash = false;
        }

        if let Some(ManagerModal::Connecting(st)) = &mut self.modal {
            if st.error.is_none() && st.host_key_prompt.is_none() {
                if let Some(started) = self.connect_started_at {
                    st.elapsed_ms = started.elapsed().as_millis() as u64;
                }
                st.blink_on = !st.blink_on;
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

    fn handle_window_close(&mut self, wid: window::Id) -> Task<Message> {
        if Some(wid) == self.manager_window {
            if self.tray.is_some() && !self.is_quitting {
                self.manager_visible = false;
                return window::set_mode(wid, window::Mode::Hidden);
            }
            return self.quit_all();
        }

        if let Some(mut srv) = self.server_windows.remove(&wid) {
            srv.closing = true;
            let sess = srv.session;
            tokio::spawn(async move {
                sess.disconnect().await;
            });
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
                for rec in newly_trusted {
                    let _ = self.vault.trust_host_key(
                        &rec.host,
                        rec.port,
                        &rec.algorithm,
                        &rec.fingerprint_sha256,
                    );
                }
                if detected_os.is_some() {
                    let _ = self.vault.update_server_os(&server_id, detected_os.clone());
                }
                self.refresh_vault_cache();

                if matches!(&self.modal, Some(ManagerModal::Connecting(st)) if st.server_id == server_id)
                {
                    self.modal = None;
                }
                self.connect_started_at = None;

                let Some(server) = self.vault.get_server(&server_id) else {
                    tokio::spawn(async move {
                        session.disconnect().await;
                    });
                    return Task::none();
                };

                let (new_wid, open_task) = window::open(server_window_settings());
                if let Ok(mut guard) = window_slot.lock() {
                    *guard = Some(new_wid);
                }

                let cached_commands = self.vault.list_commands(&server.id);
                let cached_secrets = self.vault.list_secrets(&server.id);

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
                    ServerWindowState {
                        server,
                        session,
                        active_tab: ServerTab::Dashboard,
                        dashboard: DashboardState::default(),
                        terminal: TerminalPageState::default(),
                        cached_commands,
                        cached_secrets,
                        closing: false,
                    },
                );

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
                if let Some(srv) = self.server_windows.get_mut(&window_id) {
                    srv.terminal.set_shell(&tab_id, handle);
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
                            srv.terminal.feed_output(&tab_id, &bytes);
                        }
                        SessionEvent::Shell(ShellEvent::Closed { tab_id }) => {
                            srv.terminal.mark_closed(&tab_id);
                        }
                        SessionEvent::Disconnected(reason) => {
                            if !srv.closing {
                                let name = display_server_name(&srv.server);
                                self.server_windows.remove(&wid);
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
                        let _ = self.vault.set_server_vault_key(&server_id, &key_id);
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
        }
    }

    fn handle_manager_msg(&mut self, msg: ManagerMsg) -> Task<Message> {
        match msg {
            ManagerMsg::DismissDisconnectBanner => {
                self.disconnect_banner = None;
                Task::none()
            }
            ManagerMsg::OpenMobileModal => {
                self.modal = Some(ManagerModal::Mobile);
                Task::none()
            }
            ManagerMsg::OpenSshKeysModal => {
                self.modal = Some(ManagerModal::SshKeys(SshKeysModalState::default()));
                Task::none()
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
                self.modal = Some(ManagerModal::Settings(SettingsModalState {
                    encryption_available: self.vault.is_encryption_available(),
                    data_dir: self.vault.data_dir().display().to_string(),
                }));
                Task::none()
            }
            ManagerMsg::ToggleAddForm => {
                if matches!(&self.form, Some(f) if f.editing_id.is_none()) {
                    self.form = None;
                } else {
                    self.form = Some(ServerFormState::default());
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
            ManagerMsg::SubmitForm => {
                if let Some(f) = &mut self.form {
                    match f.to_input() {
                        Ok(input) => {
                            let res = if let Some(id) = input.id.clone() {
                                self.vault.edit_server(&id, input)
                            } else {
                                self.vault.add_server(input).map(|_| ())
                            };
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
        match msg {
            ModalMsg::Noop => Task::none(),

            ModalMsg::Close => {
                self.modal = None;
                self.connect_started_at = None;
                Task::none()
            }

            ModalMsg::ConfirmDelete(server_id) => {
                let _ = self.vault.delete_server(&server_id);
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
                match self.vault.export_servers_encrypted(&pw, Some(&ids)) {
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
                let Some(content) = &st.encrypted_content else {
                    st.error = Some("Choose a .glyph backup file first.".into());
                    return Task::none();
                };
                match self.vault.decode_import_servers(content, &st.password) {
                    Ok(list) => {
                        st.selected_indices = (0..list.len()).collect();
                        st.step = ImportStep::SelectServers(list);
                        st.error = None;
                    }
                    Err(e) => st.error = Some(e.to_string()),
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
                let Some(ManagerModal::Import(st)) = &mut self.modal else {
                    return Task::none();
                };
                if let ImportStep::SelectServers(list) = &st.step {
                    let chosen: Vec<ServerEntry> = list
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| st.selected_indices.contains(i))
                        .map(|(_, s)| s.clone())
                        .collect();
                    match self.vault.import_servers(chosen) {
                        Ok(_) => {
                            self.modal = None;
                            self.refresh_vault_cache();
                        }
                        Err(e) => st.error = Some(e.to_string()),
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
                if let Some(ManagerModal::SshKeys(st)) = &mut self.modal {
                    let name = st.gen_name.trim();
                    if name.is_empty() {
                        st.error = Some("Enter a label for the new SSH key.".into());
                        return Task::none();
                    }
                    let pass = if st.gen_passphrase.is_empty() {
                        None
                    } else {
                        Some(st.gen_passphrase.as_str())
                    };
                    match self.vault.generate_ssh_key(name, st.gen_algo, pass) {
                        Ok(created) => {
                            st.gen_name.clear();
                            st.gen_passphrase.clear();
                            st.show_generate = false;
                            st.error = None;
                            st.status = Some(format!(
                                "Generated {} key \"{}\".",
                                created.algorithm, created.name
                            ));
                            self.refresh_vault_cache();
                        }
                        Err(e) => st.error = Some(e.to_string()),
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
                if let Some(ManagerModal::SshKeys(st)) = &mut self.modal {
                    let name = st.imp_name.trim();
                    if name.is_empty() {
                        st.error = Some("Enter a label for the imported SSH key.".into());
                        return Task::none();
                    }
                    let Some(pem) = &st.imp_pem else {
                        st.error = Some("Choose an OpenSSH private key file first.".into());
                        return Task::none();
                    };
                    let pass = if st.imp_passphrase.is_empty() {
                        None
                    } else {
                        Some(st.imp_passphrase.as_str())
                    };
                    match self.vault.import_ssh_key(name, pem, pass) {
                        Ok(imported) => {
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
                            self.refresh_vault_cache();
                        }
                        Err(e) => st.error = Some(e.to_string()),
                    }
                }
                Task::none()
            }
            ModalMsg::SshKeyCopyPublic(kid) => {
                if let Some(pub_key) = self.vault.get_ssh_key_public(&kid) {
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
                let Some(ManagerModal::SshKeys(st)) = &mut self.modal else {
                    return Task::none();
                };
                let (Some(key_id), Some(server_id)) =
                    (st.deploy_key_id.clone(), st.deploy_server_id.clone())
                else {
                    return Task::none();
                };
                let Some(pub_key) = self.vault.get_ssh_key_public(&key_id) else {
                    st.error = Some("SSH key not found.".into());
                    return Task::none();
                };
                let cfg = match self.vault.get_connection_config(&server_id) {
                    Ok(c) => c,
                    Err(e) => {
                        st.error = Some(e.to_string());
                        return Task::none();
                    }
                };
                let known_hosts = self.vault.known_hosts_map();
                let host_port = format!("{}:{}", cfg.host, cfg.port);
                if !known_hosts.contains_key(&host_port) {
                    st.error = Some(format!(
                        "Host key for {host_port} is not trusted yet. Connect to this server once first to verify TOFU host identity, then deploy."
                    ));
                    return Task::none();
                }

                let server_label = self
                    .vault
                    .get_server(&server_id)
                    .map(|s| display_server_name(&s))
                    .unwrap_or_else(|| host_port.clone());

                st.busy = true;
                st.error = None;
                st.status = Some(format!("Connecting to {server_label} to authorize key..."));

                let bus = app_bus_sender();
                tokio::spawn(async move {
                    let (ev_tx, mut ev_rx) = mpsc::unbounded_channel();
                    let (hk_tx, mut hk_rx) = mpsc::unbounded_channel::<HostKeyPrompt>();
                    // Known host is already trusted in known_hosts_map; reject unexpected mismatches
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
                let _ = self.vault.delete_ssh_key(&kid);
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

            ModalMsg::RemoveKnownHost(host_port) => {
                let _ = self.vault.remove_known_host(&host_port);
                Task::none()
            }
        }
    }

    fn handle_sidebar_msg(&mut self, wid: window::Id, msg: SidebarMsg) -> Task<Message> {
        match msg {
            SidebarMsg::SelectTab(tab) => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    srv.active_tab = tab;
                    if tab == ServerTab::Terminal {
                        let active = srv.terminal.active_tab_id.clone();
                        srv.terminal.focused_tab_id = active;
                    }
                }
                Task::none()
            }
            SidebarMsg::OpenMobile => {
                self.modal = Some(ManagerModal::Mobile);
                self.show_manager_window()
            }
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
            TerminalPageMsg::RunSnippet(cmd) => {
                srv.terminal.show_snippets = false;
                let target = srv.terminal.focused_tab_id.clone();
                let mut payload = cmd.into_bytes();
                payload.push(b'\r');
                srv.terminal.send_input(&target, payload);
            }
            TerminalPageMsg::InjectSecret(secret_id) => {
                srv.terminal.show_secrets = false;
                if let Ok(Some(val)) = self.vault.get_secret_value(&secret_id) {
                    let target = srv.terminal.focused_tab_id.clone();
                    srv.terminal.send_input(&target, val.into_bytes());
                }
            }
            TerminalPageMsg::CanvasAction(tid, ev) => match ev {
                TerminalEvent::Input(bytes) => {
                    srv.terminal.send_input(&tid, bytes);
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
                        srv.terminal.send_input(&tid, normalized.into_bytes());
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

    fn start_connect(&mut self, server_id: String) {
        let Some(summary) = self.vault.get_server(&server_id) else {
            return;
        };
        let cfg = match self.vault.get_connection_config(&server_id) {
            Ok(c) => c,
            Err(e) => {
                self.disconnect_banner = Some(e.to_string());
                return;
            }
        };
        let known_hosts = self.vault.known_hosts_map();

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
        if let Some(srv) = self.server_windows.get(&wid) {
            return self.view_server_window(wid, srv);
        }

        let title_el = titlebar::view("Glyph", true).map(move |m| Message::TitleBar(wid, m));
        if self.show_splash {
            return column![title_el, splash::view()]
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }

        let base = manager::view(
            &self.servers,
            &self.ssh_keys,
            self.form.as_ref(),
            self.disconnect_banner.as_deref(),
        )
        .map(Message::Manager);

        let body: Element<'_, Message> = if let Some(modal) = &self.modal {
            let overlay = modals::view(
                modal,
                &self.servers,
                &self.ssh_keys,
                self.vault.list_known_hosts(),
            )
            .map(Message::Modal);
            stack![base, overlay]
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            base
        };

        column![title_el, body]
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

        let main_pane: Element<'a, Message> = match srv.active_tab {
            ServerTab::Dashboard => {
                dashboard::view(&srv.dashboard, &srv.server).map(move |m| Message::Dashboard(wid, m))
            }
            ServerTab::Terminal => {
                terminal_page::view(&srv.terminal, &srv.cached_commands, &srv.cached_secrets)
                    .map(move |m| Message::Terminal(wid, m))
            }
            other => view_milestone2_placeholder(other, &srv.server),
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

        column![title_el, content_row]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

fn view_milestone2_placeholder<'a>(
    tab: ServerTab,
    server: &'a ServerSummary,
) -> Element<'a, Message> {
    let (title, summary, icon_el): (&'static str, &'static str, Element<'a, Message>) = match tab
    {
        ServerTab::Sftp => (
            "SFTP File Explorer",
            "Dual-pane directory browser, drag-and-drop upload/download, inline permission editor, and remote text viewer via pure-Rust russh-sftp.",
            icons::folder_open(44.0, BRAND_400).into(),
        ),
        ServerTab::Commands => (
            "Saved Commands Manager",
            "Create, organize, and trigger frequently used remote shell scripts. Note: Your saved Snippets dropdown is already live in the Terminal toolbar!",
            icons::code(44.0, BRAND_400).into(),
        ),
        ServerTab::Secrets => (
            "Per-Server Secrets Vault",
            "Store encrypted API tokens and credentials per server and export/import .glyph secret bundles. Quick Secrets injection is already active in Terminal!",
            icons::lock(44.0, BRAND_400).into(),
        ),
        ServerTab::Tunnels => (
            "SSH Port Forwarding & Tunnels",
            "Managed Local (-L), Remote (-R), and Dynamic SOCKS5 (-D) tunnels over your encrypted SSH session.",
            icons::network(44.0, BRAND_400).into(),
        ),
        ServerTab::Containers => (
            "Docker & Podman Containers",
            "Live container table with start, stop, restart, log tailing, and instant exec shell launch.",
            icons::cube(44.0, BRAND_400).into(),
        ),
        ServerTab::Vnc => (
            "VNC Remote Desktop over SSH",
            "Pure-Rust RFB viewer tunneled natively over SSH direct-tcpip with zero plaintext exposure.",
            icons::monitor(44.0, BRAND_400).into(),
        ),
        ServerTab::Dashboard | ServerTab::Terminal => unreachable!(),
    };

    let card = container(
        column![
            icon_el,
            container(
                text("MILESTONE 2 MODULE")
                    .size(10)
                    .font(FONT_MONO_BOLD)
                    .color(BRAND_300)
            )
            .padding([3, 10])
            .style(pill_badge(
                rgba(BRAND_500, 0.16),
                rgba(BRAND_500, 0.35),
                BRAND_300,
            )),
            text(title)
                .size(22)
                .font(FONT_MONO_BOLD)
                .color(TEXT_PRIMARY),
            text(format!(
                "Target: {} ({}@{}:{})",
                display_server_name(server),
                server.username,
                server.host,
                server.port
            ))
            .size(12)
            .font(FONT_MONO)
            .color(TEXT_MUTED),
            container(text(summary).size(13).color(TEXT_SECONDARY))
                .max_width(480)
                .padding([4, 0]),
        ]
        .spacing(12)
        .align_x(alignment::Horizontal::Center),
    )
    .padding(36)
    .style(|_| {
        let mut st = dark_card(&glyph_theme());
        st.border = Border {
            color: DARK_700,
            width: 1.0,
            radius: 16.0.into(),
        };
        st
    });

    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(dark_900_bg)
        .into()
}

fn display_server_name(srv: &ServerSummary) -> String {
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

fn server_window_settings() -> window::Settings {
    window::Settings {
        size: Size::new(1280.0, 820.0),
        min_size: Some(Size::new(960.0, 640.0)),
        decorations: false,
        exit_on_close_request: false,
        icon: load_window_icon(),
        ..Default::default()
    }
}

fn clipboard_set(txt: String) {
    if let Ok(mut cb) = arboard::Clipboard::new() {
        let _ = cb.set_text(txt);
    }
}

fn clipboard_get() -> Option<String> {
    arboard::Clipboard::new().ok().and_then(|mut cb| cb.get_text().ok())
}
