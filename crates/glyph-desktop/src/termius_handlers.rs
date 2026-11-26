use crate::app::{
    app_bus_sender, clipboard_get, clipboard_set, display_server_name, server_window_settings,
    spawn_sftp_readdir_peer, BusEvent, GlyphApp, Message,
};
use crate::ui::commands_page::{CommandsPageMsg, CommandsSubView};
use crate::ui::lock_screen::{LockScreenMsg, LockScreenState};
use crate::ui::manager::ManagerMsg;
use crate::ui::modals::{ManagerModal, ModalMsg, SettingsModalState, SettingsTab};
use crate::ui::palette::{filtered_indices, PaletteAction, PaletteItem, PaletteMsg};
use crate::ui::quick_terminal::{
    QuickProtocol, QuickTerminalMsg, QuickTerminalState, QuickTransport,
};
use crate::ui::sftp_page::{
    list_local_dir, local_parent_path, DualPaneMode, QueuedTransfer, SftpPageMsg, TransferStatus,
};
use crate::ui::sidebar::ServerTab;
use crate::ui::terminal_page::BroadcastMode;
use crate::ui::termius_modals::{
    GroupRunModalState, GroupRunTargetResult, HostImportModalState, IdentitiesModalState,
    SnippetVarsModalState, TermiusModal, TermiusModalMsg,
};
use crate::ui::tunnels_page::{TunnelsPageMsg, UiTunnelDirection};
use glyph_core::cloud_import::fetch_cloud_hosts;
use glyph_core::importers::{
    export_ssh_config, parse_hosts_csv, parse_putty_reg, parse_ssh_config,
};
use glyph_core::local_pty::{LocalPtyEvent, LocalPtySession};
use glyph_core::models::{
    HighlightRule, HostGroup, IdentityInput, SavedTunnelRule, TunnelDirection,
};
use glyph_core::serial::{list_serial_ports, SerialConfig, SerialEvent, SerialSession};
use glyph_core::sftp::{join_remote_path, sftp_transfer_between_sessions, TransferKind};
use glyph_core::ssh::{HostKeyPrompt, SessionEvent, SshSession};
use glyph_core::sync::run_e2ee_sync_locked;
use glyph_core::telnet::{TelnetEvent, TelnetSession};
use glyph_core::vault::{extract_template_vars, render_template_cmd};
use glyph_terminal::{compile_highlight_rules, export_plain_log, SessionRecorder, TerminalEvent};
use iced::window;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

impl GlyphApp {
    pub(crate) fn reload_termius_caches(&mut self) {
        let (groups, identities, packages, hl_rules, lock_cfg) = self.with_vault(|v| {
            (
                v.list_groups().to_vec(),
                v.list_identities(),
                v.list_snippet_packages().to_vec(),
                v.list_highlight_rules().to_vec(),
                v.app_lock_config().clone(),
            )
        });
        self.compiled_highlights = compile_highlight_rules(
            hl_rules
                .iter()
                .map(|r| (r.pattern.as_str(), r.color_hex.as_str(), r.enabled)),
        );
        self.groups = groups;
        self.identities = identities;
        self.snippet_packages = packages;
        self.highlight_rules = hl_rules;
        self.app_lock_cfg = lock_cfg;
        self.rebuild_palette();
    }

    pub(crate) fn rebuild_palette(&mut self) {
        let mut items = Vec::new();
        for (&wid, srv) in &self.server_windows {
            items.push(PaletteItem {
                title: format!("Active: {}", display_server_name(&srv.server)),
                subtitle: format!(
                    "{}@{}:{}",
                    srv.server.username, srv.server.host, srv.server.port
                ),
                badge: "Window".to_string(),
                action: PaletteAction::FocusServerWindow(wid),
            });
        }
        for s in &self.servers {
            items.push(PaletteItem {
                title: display_server_name(s),
                subtitle: format!("{}@{}:{} {}", s.username, s.host, s.port, s.tags.join(" ")),
                badge: "SSH Host".to_string(),
                action: PaletteAction::ConnectServer(s.id.clone()),
            });
        }
        items.push(PaletteItem {
            title: "Filter: All Hosts".to_string(),
            subtitle: "Clear group filter and show every saved server".to_string(),
            badge: "Group".to_string(),
            action: PaletteAction::FilterGroup(None),
        });
        for g in &self.groups {
            items.push(PaletteItem {
                title: format!("Group: {}", g.name),
                subtitle: g.tags.join(", "),
                badge: "Group".to_string(),
                action: PaletteAction::FilterGroup(Some(g.id.clone())),
            });
        }
        items.push(PaletteItem {
            title: "New Local Terminal".to_string(),
            subtitle: "Open native interactive local shell (PowerShell / Bash)".to_string(),
            badge: "Quick Launch".to_string(),
            action: PaletteAction::OpenLocalTerminal,
        });
        items.push(PaletteItem {
            title: "Serial / UART Console".to_string(),
            subtitle: "Connect to COM / ttyUSB / ttyACM hardware console".to_string(),
            badge: "Quick Launch".to_string(),
            action: PaletteAction::OpenSerial,
        });
        items.push(PaletteItem {
            title: "Telnet Client".to_string(),
            subtitle: "Open raw RFC-854 Telnet session".to_string(),
            badge: "Quick Launch".to_string(),
            action: PaletteAction::OpenTelnet,
        });
        items.push(PaletteItem {
            title: "Manage Reusable Identities".to_string(),
            subtitle: "Shared credentials & keys inherited by hosts or groups".to_string(),
            badge: "Vault".to_string(),
            action: PaletteAction::OpenIdentities,
        });
        items.push(PaletteItem {
            title: "Manage SSH Keys".to_string(),
            subtitle: "Generate, import, and deploy keys via ssh-copy-id".to_string(),
            badge: "Vault".to_string(),
            action: PaletteAction::OpenSshKeys,
        });
        items.push(PaletteItem {
            title: "Import / Export Hosts".to_string(),
            subtitle: "OpenSSH ~/.ssh/config, PuTTY .reg, CSV, DigitalOcean, Hetzner, Tailscale"
                .to_string(),
            badge: "Import".to_string(),
            action: PaletteAction::OpenHostImport,
        });
        items.push(PaletteItem {
            title: "Settings: E2EE Vault Sync".to_string(),
            subtitle: "Zero-knowledge encrypted Local Folder or WebDAV sync".to_string(),
            badge: "Sync".to_string(),
            action: PaletteAction::OpenSyncSettings,
        });
        items.push(PaletteItem {
            title: "Settings & Security".to_string(),
            subtitle: "MCP integrations, Master Passcode, Highlight Rules, Updates".to_string(),
            badge: "Settings".to_string(),
            action: PaletteAction::OpenSettings,
        });
        items.push(PaletteItem {
            title: "Open Activity Logs".to_string(),
            subtitle: "Inspect live success, info, warning, and error events (Ctrl+Shift+L)"
                .to_string(),
            badge: "Logs".to_string(),
            action: PaletteAction::OpenLogs,
        });
        if self.app_lock_cfg.enabled {
            items.push(PaletteItem {
                title: "Lock Glyph Vault Now".to_string(),
                subtitle: "Mask all windows behind Master Passcode screen".to_string(),
                badge: "Security".to_string(),
                action: PaletteAction::LockVault,
            });
        }
        self.palette.items = items;
    }

    pub(crate) fn populate_settings_termius(&self, st: &mut SettingsModalState) {
        self.with_vault(|v| {
            let lock = v.app_lock_config();
            st.lock_enabled = lock.enabled;
            st.lock_idle_mins = lock.idle_timeout_mins.to_string();
            st.highlight_rules = v.list_highlight_rules().to_vec();
            let sc = v.sync_config();
            st.sync_backend = sc.backend;
            st.sync_folder_path = sc.folder_path.clone();
            st.sync_webdav_url = sc.webdav_url.clone();
            st.sync_webdav_user = sc.webdav_username.clone();
            st.sync_last_at = sc.last_sync_at.clone();
        });
    }

    pub(crate) fn open_settings_modal_tab(&mut self, tab: SettingsTab) {
        let (enc, dir) = self.with_vault(|v| {
            (
                v.is_encryption_available(),
                v.data_dir().display().to_string(),
            )
        });
        let mut st = SettingsModalState::new(enc, dir);
        st.active_tab = tab;
        self.populate_settings_termius(&mut st);
        self.modal = Some(ManagerModal::Settings(Box::new(st)));
    }

    pub(crate) fn handle_lock_screen_msg(&mut self, msg: LockScreenMsg) -> iced::Task<Message> {
        match msg {
            LockScreenMsg::InputChanged(s) => {
                self.lock_screen.input = s;
                self.lock_screen.error = None;
            }
            LockScreenMsg::Submit => {
                let ok = self.with_vault(|v| v.verify_app_lock_passcode(&self.lock_screen.input));
                if ok {
                    self.locked = false;
                    self.lock_screen = LockScreenState::default();
                    self.last_activity = Instant::now();
                } else {
                    self.lock_screen.input.clear();
                    self.lock_screen.error = Some("Incorrect master passcode.".to_string());
                }
            }
        }
        iced::Task::none()
    }

    pub(crate) fn handle_palette_msg(&mut self, msg: PaletteMsg) -> iced::Task<Message> {
        match msg {
            PaletteMsg::Noop => iced::Task::none(),
            PaletteMsg::Close => {
                self.palette.open = false;
                self.palette.query.clear();
                self.palette.selected_idx = 0;
                iced::Task::none()
            }
            PaletteMsg::QueryChanged(q) => {
                self.palette.query = q;
                self.palette.selected_idx = 0;
                iced::Task::none()
            }
            PaletteMsg::ActivateSelected => {
                let idx = self.palette.selected_idx;
                self.activate_palette_pos(idx)
            }
            PaletteMsg::ActivateIndex(pos) => self.activate_palette_pos(pos),
        }
    }

    pub(crate) fn handle_palette_nav(&mut self, down: bool) {
        if !self.palette.open {
            return;
        }
        let len = filtered_indices(&self.palette).len();
        if len == 0 {
            self.palette.selected_idx = 0;
            return;
        }
        if down {
            self.palette.selected_idx = (self.palette.selected_idx + 1) % len;
        } else if self.palette.selected_idx == 0 {
            self.palette.selected_idx = len - 1;
        } else {
            self.palette.selected_idx -= 1;
        }
    }

    fn activate_palette_pos(&mut self, pos: usize) -> iced::Task<Message> {
        let matches = filtered_indices(&self.palette);
        let Some(&item_idx) = matches.get(pos) else {
            return iced::Task::none();
        };
        let action = self.palette.items[item_idx].action.clone();
        self.palette.open = false;
        self.palette.query.clear();
        self.palette.selected_idx = 0;

        match action {
            PaletteAction::ConnectServer(sid) => {
                self.start_connect(sid);
                self.show_manager_window()
            }
            PaletteAction::FocusServerWindow(wid) => self.reveal_server_window(wid),
            PaletteAction::FilterGroup(gid) => {
                self.selected_group_filter = gid;
                self.show_manager_window()
            }
            PaletteAction::OpenLocalTerminal => self.open_quick_terminal(QuickProtocol::LocalShell),
            PaletteAction::OpenSerial => self.open_quick_terminal(QuickProtocol::Serial),
            PaletteAction::OpenTelnet => self.open_quick_terminal(QuickProtocol::Telnet),
            PaletteAction::OpenIdentities => {
                self.termius_modal_window = self.manager_window;
                self.termius_modal =
                    Some(TermiusModal::Identities(IdentitiesModalState::default()));
                self.show_manager_window()
            }
            PaletteAction::OpenSshKeys => {
                let task = self.show_manager_window();
                let _ = self.handle_manager_msg(ManagerMsg::OpenSshKeysModal);
                task
            }
            PaletteAction::OpenHostImport => {
                self.termius_modal_window = self.manager_window;
                self.termius_modal =
                    Some(TermiusModal::ImportHosts(HostImportModalState::default()));
                self.show_manager_window()
            }
            PaletteAction::OpenSettings => {
                self.open_settings_modal_tab(SettingsTab::AiAgents);
                self.show_manager_window()
            }
            PaletteAction::OpenSyncSettings => {
                self.open_settings_modal_tab(SettingsTab::Sync);
                self.show_manager_window()
            }
            PaletteAction::OpenLogs => {
                self.logger.open_panel();
                iced::Task::none()
            }
            PaletteAction::LockVault => {
                self.locked = true;
                self.lock_screen = LockScreenState::default();
                iced::Task::none()
            }
        }
    }

    pub(crate) fn open_quick_terminal(&mut self, protocol: QuickProtocol) -> iced::Task<Message> {
        let (wid, open_task) = window::open(server_window_settings());
        let mut st = QuickTerminalState::new(protocol);

        if protocol == QuickProtocol::LocalShell {
            match LocalPtySession::spawn(st.cols, st.rows, None) {
                Ok((handle, mut rx)) => {
                    st.transport = QuickTransport::Local(handle);
                    let bus = app_bus_sender();
                    tokio::spawn(async move {
                        while let Some(ev) = rx.recv().await {
                            match ev {
                                LocalPtyEvent::Data(bytes) => {
                                    let _ = bus.send(BusEvent::QuickTerminalData(wid, bytes));
                                }
                                LocalPtyEvent::Closed(reason) => {
                                    let _ = bus.send(BusEvent::QuickTerminalClosed(wid, reason));
                                    break;
                                }
                            }
                        }
                    });
                }
                Err(e) => {
                    st.error = Some(format!("Failed to launch local shell: {e}"));
                }
            }
        }

        self.quick_windows.insert(wid, st);
        iced::Task::batch([open_task.discard(), window::gain_focus(wid)])
    }

    pub(crate) fn handle_quick_terminal_msg(
        &mut self,
        wid: window::Id,
        msg: QuickTerminalMsg,
    ) -> iced::Task<Message> {
        match msg {
            QuickTerminalMsg::StartDrag => return window::drag(wid),
            QuickTerminalMsg::MinimizeWindow => return window::minimize(wid, true),
            QuickTerminalMsg::MaximizeWindow => return window::toggle_maximize(wid),
            QuickTerminalMsg::CloseWindow => return self.handle_window_close(wid),
            _ => {}
        }

        let Some(qt) = self.quick_windows.get_mut(&wid) else {
            return iced::Task::none();
        };

        match msg {
            QuickTerminalMsg::SelectTheme(t) => qt.theme = t,
            QuickTerminalMsg::FontSmaller => qt.font_size = (qt.font_size - 1.0).max(10.0),
            QuickTerminalMsg::FontLarger => qt.font_size = (qt.font_size + 1.0).min(26.0),
            QuickTerminalMsg::ToggleHighlights => qt.highlights_enabled = !qt.highlights_enabled,
            QuickTerminalMsg::ToggleRecording => {
                if qt.recorder.is_some() {
                    let path = qt
                        .recorder
                        .take()
                        .map(|r| r.path().display().to_string())
                        .unwrap_or_default();
                    qt.status = Some(format!("Saved .cast recording: {path}"));
                } else {
                    match SessionRecorder::start(
                        &qt.title(),
                        u32::from(qt.cols),
                        u32::from(qt.rows),
                    ) {
                        Ok(rec) => {
                            let path = rec.path().display().to_string();
                            qt.recorder = Some(rec);
                            qt.error = None;
                            qt.status = Some(format!("Recording asciinema .cast -> {path}"));
                        }
                        Err(e) => qt.error = Some(format!("Recording failed: {e}")),
                    }
                }
            }
            QuickTerminalMsg::ExportPlainLog => {
                let txt = qt.emulator.all_text();
                match export_plain_log(&qt.title(), &txt) {
                    Ok(path) => {
                        qt.error = None;
                        qt.status = Some(format!("Exported plain log -> {}", path.display()));
                    }
                    Err(e) => qt.error = Some(format!("Log export failed: {e}")),
                }
            }
            QuickTerminalMsg::ToggleSearch => {
                qt.show_search = !qt.show_search;
                if !qt.show_search {
                    qt.search_text.clear();
                    qt.emulator.clear_search();
                }
            }
            QuickTerminalMsg::SearchInput(q) => {
                qt.search_text = q.clone();
                if q.is_empty() {
                    qt.emulator.clear_search();
                } else {
                    qt.emulator.search(&q, false);
                }
            }
            QuickTerminalMsg::SearchNext => {
                if !qt.search_text.is_empty() {
                    qt.emulator.search(&qt.search_text, true);
                }
            }
            QuickTerminalMsg::SearchPrev => {
                if !qt.search_text.is_empty() {
                    qt.emulator.search(&qt.search_text, false);
                }
            }
            QuickTerminalMsg::ClearScreen => qt.emulator.clear(),
            QuickTerminalMsg::ReconnectLocal => {
                qt.transport.close();
                qt.transport = QuickTransport::Disconnected;
                qt.error = None;
                match LocalPtySession::spawn(qt.cols, qt.rows, None) {
                    Ok((handle, mut rx)) => {
                        qt.transport = QuickTransport::Local(handle);
                        let bus = app_bus_sender();
                        tokio::spawn(async move {
                            while let Some(ev) = rx.recv().await {
                                match ev {
                                    LocalPtyEvent::Data(bytes) => {
                                        let _ = bus.send(BusEvent::QuickTerminalData(wid, bytes));
                                    }
                                    LocalPtyEvent::Closed(reason) => {
                                        let _ =
                                            bus.send(BusEvent::QuickTerminalClosed(wid, reason));
                                        break;
                                    }
                                }
                            }
                        });
                    }
                    Err(e) => qt.error = Some(format!("Restart failed: {e}")),
                }
            }
            QuickTerminalMsg::RefreshSerialPorts => {
                qt.serial_ports = list_serial_ports();
                if qt.serial_port_input.is_empty() {
                    if let Some(first) = qt.serial_ports.first() {
                        qt.serial_port_input = first.name.clone();
                    }
                }
            }
            QuickTerminalMsg::SelectSerialPort(p) => qt.serial_port_input = p.name,
            QuickTerminalMsg::SerialPortInput(v) => qt.serial_port_input = v,
            QuickTerminalMsg::SelectBaud(b) => qt.serial_baud = b,
            QuickTerminalMsg::SelectDataBits(b) => qt.serial_data_bits = b,
            QuickTerminalMsg::SelectParity(p) => qt.serial_parity = p,
            QuickTerminalMsg::SelectStopBits(s) => qt.serial_stop_bits = s,
            QuickTerminalMsg::ConnectSerial => {
                qt.transport.close();
                qt.transport = QuickTransport::Disconnected;
                let cfg = SerialConfig {
                    port_name: qt.serial_port_input.clone(),
                    baud_rate: qt.serial_baud,
                    data_bits: qt.serial_data_bits,
                    parity: qt.serial_parity.clone(),
                    stop_bits: qt.serial_stop_bits,
                };
                match SerialSession::open(&cfg) {
                    Ok((handle, mut rx)) => {
                        qt.transport = QuickTransport::Serial(handle);
                        qt.error = None;
                        qt.status = Some(format!(
                            "Connected {} @ {} baud",
                            cfg.port_name, cfg.baud_rate
                        ));
                        let bus = app_bus_sender();
                        tokio::spawn(async move {
                            while let Some(ev) = rx.recv().await {
                                match ev {
                                    SerialEvent::Data(bytes) => {
                                        let _ = bus.send(BusEvent::QuickTerminalData(wid, bytes));
                                    }
                                    SerialEvent::Closed(reason) => {
                                        let _ =
                                            bus.send(BusEvent::QuickTerminalClosed(wid, reason));
                                        break;
                                    }
                                }
                            }
                        });
                    }
                    Err(e) => qt.error = Some(e.to_string()),
                }
            }
            QuickTerminalMsg::TelnetHostInput(v) => qt.telnet_host = v,
            QuickTerminalMsg::TelnetPortInput(v) => qt.telnet_port = v,
            QuickTerminalMsg::ConnectTelnet => {
                let port: u16 = match qt.telnet_port.trim().parse() {
                    Ok(p) if p > 0 => p,
                    _ => {
                        qt.error = Some("Enter a valid Telnet port (1-65535).".to_string());
                        return iced::Task::none();
                    }
                };
                let host = qt.telnet_host.trim().to_string();
                let cols = qt.cols;
                let rows = qt.rows;
                qt.transport.close();
                qt.transport = QuickTransport::Disconnected;
                qt.error = None;
                qt.status = Some(format!("Connecting via Telnet to {host}:{port}..."));
                tokio::spawn(async move {
                    let bus = app_bus_sender();
                    match TelnetSession::connect(&host, port, cols, rows).await {
                        Ok((handle, mut rx)) => {
                            let bus_d = bus.clone();
                            tokio::spawn(async move {
                                while let Some(ev) = rx.recv().await {
                                    match ev {
                                        TelnetEvent::Data(bytes) => {
                                            let _ =
                                                bus_d.send(BusEvent::QuickTerminalData(wid, bytes));
                                        }
                                        TelnetEvent::Closed(reason) => {
                                            let _ = bus_d.send(BusEvent::QuickTerminalClosed(
                                                wid,
                                                Some(reason),
                                            ));
                                            break;
                                        }
                                    }
                                }
                            });
                            let _ = bus.send(BusEvent::QuickTelnetConnected(wid, Ok(handle)));
                        }
                        Err(e) => {
                            let _ =
                                bus.send(BusEvent::QuickTelnetConnected(wid, Err(e.to_string())));
                        }
                    }
                });
            }
            QuickTerminalMsg::DisconnectTransport => {
                qt.transport.close();
                qt.transport = QuickTransport::Disconnected;
                qt.status = Some("Disconnected".to_string());
            }
            QuickTerminalMsg::CanvasAction(ev) => match ev {
                TerminalEvent::Input(bytes) => qt.transport.write(bytes),
                TerminalEvent::Resized { cols, rows } => {
                    if qt.emulator.resize(cols, rows) {
                        qt.cols = cols as u16;
                        qt.rows = rows as u16;
                        qt.transport.resize(qt.cols, qt.rows);
                    }
                }
                TerminalEvent::CopyText(txt) => clipboard_set(txt),
                TerminalEvent::PasteRequested => {
                    if let Some(txt) = clipboard_get() {
                        let norm = txt.replace("\r\n", "\r").replace('\n', "\r");
                        qt.transport.write(norm.into_bytes());
                    }
                }
                TerminalEvent::ToggleSearch => qt.show_search = !qt.show_search,
                TerminalEvent::FocusRequested => qt.focused = true,
                TerminalEvent::StartSelection { col, row } => qt.emulator.start_selection(col, row),
                TerminalEvent::UpdateSelection { col, row } => {
                    qt.emulator.update_selection(col, row)
                }
                TerminalEvent::WheelScroll(delta) => qt.emulator.scroll_display(delta),
            },
            QuickTerminalMsg::StartDrag
            | QuickTerminalMsg::MinimizeWindow
            | QuickTerminalMsg::MaximizeWindow
            | QuickTerminalMsg::CloseWindow => {}
        }

        iced::Task::none()
    }

    pub(crate) fn send_terminal_payload(
        &mut self,
        origin_wid: window::Id,
        origin_tab_id: &str,
        bytes: Vec<u8>,
    ) {
        let Some(mode) = self
            .server_windows
            .get(&origin_wid)
            .map(|s| s.terminal.broadcast_mode)
        else {
            return;
        };

        match mode {
            BroadcastMode::Off => {
                if let Some(srv) = self.server_windows.get(&origin_wid) {
                    srv.terminal.send_input(origin_tab_id, bytes);
                }
            }
            BroadcastMode::Tabs => {
                if let Some(srv) = self.server_windows.get(&origin_wid) {
                    for tab in &srv.terminal.tabs {
                        srv.terminal.send_input(&tab.id, bytes.clone());
                    }
                }
            }
            BroadcastMode::AllWindows => {
                for srv in self.server_windows.values() {
                    let target = if srv.terminal.focused_tab_id.is_empty() {
                        srv.terminal.active_tab_id.as_str()
                    } else {
                        srv.terminal.focused_tab_id.as_str()
                    };
                    srv.terminal.send_input(target, bytes.clone());
                }
            }
        }
    }

    pub(crate) fn trigger_snippet_or_prompt(
        &mut self,
        wid: window::Id,
        snippet_id: Option<u64>,
        title: String,
        cmd: String,
    ) {
        let vars = extract_template_vars(&cmd);
        if !vars.is_empty() {
            self.termius_modal_window = Some(wid);
            self.termius_modal = Some(TermiusModal::SnippetVars(SnippetVarsModalState {
                title,
                template: cmd,
                vars: vars.into_iter().map(|v| (v, String::new())).collect(),
                window_id: wid,
                snippet_id,
                error: None,
            }));
            return;
        }
        self.execute_snippet_now(wid, snippet_id, cmd);
    }

    fn execute_snippet_now(&mut self, wid: window::Id, snippet_id: Option<u64>, cmd: String) {
        let (sid_opt, focused_tab) = if let Some(srv) = self.server_windows.get(&wid) {
            (
                Some(srv.server.id.clone()),
                srv.terminal.focused_tab_id.clone(),
            )
        } else {
            (None, String::new())
        };

        if let (Some(sid), Some(cmd_id)) = (sid_opt, snippet_id) {
            let updated = self.with_vault(|v| {
                let _ = v.increment_command_use(&sid, cmd_id);
                v.list_commands(&sid)
            });
            if let Some(srv) = self.server_windows.get_mut(&wid) {
                srv.cached_commands = updated;
            }
        }

        let mut payload = cmd.into_bytes();
        payload.push(b'\r');
        self.send_terminal_payload(wid, &focused_tab, payload);
        if let Some(srv) = self.server_windows.get_mut(&wid) {
            srv.active_tab = ServerTab::Terminal;
        }
    }

    pub(crate) fn init_server_termius_hooks(&mut self, wid: window::Id) {
        let Some(srv) = self.server_windows.get_mut(&wid) else {
            return;
        };
        if srv.server.auto_log && !srv.terminal.recorders.contains_key("tab-main") {
            let label = display_server_name(&srv.server);
            if let Ok(rec) = SessionRecorder::start(&label, 100, 30) {
                srv.terminal.notice =
                    Some(format!("Auto-recording .cast -> {}", rec.path().display()));
                srv.terminal.recorders.insert("tab-main".to_string(), rec);
            }
        }

        let auto_rules: Vec<SavedTunnelRule> = srv
            .server
            .saved_tunnels
            .iter()
            .filter(|r| r.auto_start)
            .cloned()
            .collect();
        if !auto_rules.is_empty() {
            let sess = srv.session.clone();
            tokio::spawn(async move {
                for rule in auto_rules {
                    match rule.direction {
                        TunnelDirection::Local => {
                            let _ = sess
                                .start_local_tunnel(
                                    rule.local_port,
                                    rule.remote_host,
                                    rule.remote_port,
                                    rule.protocol,
                                )
                                .await;
                        }
                        TunnelDirection::Remote => {
                            let _ = sess
                                .start_remote_tunnel(
                                    "0.0.0.0".to_string(),
                                    rule.remote_port,
                                    rule.remote_host,
                                    rule.local_port,
                                )
                                .await;
                        }
                        TunnelDirection::Dynamic => {
                            let _ = sess.start_dynamic_socks5_tunnel(rule.local_port).await;
                        }
                    }
                }
                let list = sess.list_tunnels().await;
                let _ = app_bus_sender().send(BusEvent::TunnelsListed(wid, list));
            });
        }
    }

    pub(crate) fn handle_termius_modal_msg(&mut self, msg: TermiusModalMsg) -> iced::Task<Message> {
        match msg {
            TermiusModalMsg::Noop => iced::Task::none(),
            TermiusModalMsg::Close => {
                self.termius_modal = None;
                self.termius_modal_window = None;
                iced::Task::none()
            }

            // ── Identities CRUD ───────────────────────────────────────────────
            TermiusModalMsg::IdentitySelectEdit(id) => {
                if let Some(ident) = self.identities.iter().find(|i| i.id == id).cloned() {
                    if let Some(TermiusModal::Identities(st)) = &mut self.termius_modal {
                        st.editing_id = Some(ident.id);
                        st.name = ident.name;
                        st.username = ident.username;
                        st.auth_method = ident.auth_method;
                        st.password.clear();
                        st.ssh_key_id = ident.ssh_key_id;
                        st.private_key_path = ident.private_key_path;
                        st.passphrase.clear();
                        st.status = None;
                        st.error = None;
                    }
                }
                iced::Task::none()
            }
            TermiusModalMsg::IdentityNew => {
                if let Some(TermiusModal::Identities(st)) = &mut self.termius_modal {
                    *st = IdentitiesModalState::default();
                }
                iced::Task::none()
            }
            TermiusModalMsg::IdentityName(v) => {
                if let Some(TermiusModal::Identities(st)) = &mut self.termius_modal {
                    st.name = v;
                }
                iced::Task::none()
            }
            TermiusModalMsg::IdentityUsername(v) => {
                if let Some(TermiusModal::Identities(st)) = &mut self.termius_modal {
                    st.username = v;
                }
                iced::Task::none()
            }
            TermiusModalMsg::IdentityAuthMethod(a) => {
                if let Some(TermiusModal::Identities(st)) = &mut self.termius_modal {
                    st.auth_method = a;
                }
                iced::Task::none()
            }
            TermiusModalMsg::IdentityPassword(v) => {
                if let Some(TermiusModal::Identities(st)) = &mut self.termius_modal {
                    st.password = v;
                }
                iced::Task::none()
            }
            TermiusModalMsg::IdentitySshKey(k) => {
                if let Some(TermiusModal::Identities(st)) = &mut self.termius_modal {
                    st.ssh_key_id = k;
                }
                iced::Task::none()
            }
            TermiusModalMsg::IdentityKeyPath(v) => {
                if let Some(TermiusModal::Identities(st)) = &mut self.termius_modal {
                    st.private_key_path = v;
                }
                iced::Task::none()
            }
            TermiusModalMsg::IdentityBrowseKey => iced::Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .set_title("Select Identity Private Key")
                        .pick_file()
                        .await
                        .map(|h| h.path().display().to_string())
                },
                Message::IdentityKeyFilePicked,
            ),
            TermiusModalMsg::IdentityPassphrase(v) => {
                if let Some(TermiusModal::Identities(st)) = &mut self.termius_modal {
                    st.passphrase = v;
                }
                iced::Task::none()
            }
            TermiusModalMsg::IdentitySave => {
                let Some(TermiusModal::Identities(st)) = &mut self.termius_modal else {
                    return iced::Task::none();
                };
                let input = IdentityInput {
                    name: st.name.clone(),
                    username: st.username.clone(),
                    auth_method: st.auth_method,
                    password: (!st.password.is_empty()).then(|| st.password.clone()),
                    ssh_key_id: st.ssh_key_id.clone(),
                    private_key_path: st.private_key_path.clone(),
                    passphrase: (!st.passphrase.is_empty()).then(|| st.passphrase.clone()),
                };
                let editing = st.editing_id.clone();
                let res = self
                    .vault
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Vault busy"))
                    .and_then(|mut v| {
                        if let Some(id) = editing {
                            v.edit_identity(&id, input)?;
                            Ok("Updated identity".to_string())
                        } else {
                            v.add_identity(input)?;
                            Ok("Saved new identity".to_string())
                        }
                    });
                match res {
                    Ok(msg) => {
                        if let Some(TermiusModal::Identities(st)) = &mut self.termius_modal {
                            st.error = None;
                            st.status = Some(msg);
                            st.password.clear();
                            st.passphrase.clear();
                        }
                        self.reload_termius_caches();
                    }
                    Err(e) => {
                        if let Some(TermiusModal::Identities(st)) = &mut self.termius_modal {
                            st.error = Some(e.to_string());
                        }
                    }
                }
                iced::Task::none()
            }
            TermiusModalMsg::IdentityDelete(id) => {
                let res = self.with_vault(|v| v.delete_identity(&id));
                match res {
                    Ok(()) => {
                        if let Some(TermiusModal::Identities(st)) = &mut self.termius_modal {
                            *st = IdentitiesModalState::default();
                            st.status = Some("Identity deleted".to_string());
                        }
                        self.reload_termius_caches();
                    }
                    Err(e) => {
                        if let Some(TermiusModal::Identities(st)) = &mut self.termius_modal {
                            st.error = Some(e.to_string());
                        }
                    }
                }
                iced::Task::none()
            }

            // ── Host Groups CRUD ──────────────────────────────────────────────
            TermiusModalMsg::GroupName(v) => {
                if let Some(TermiusModal::Group(st)) = &mut self.termius_modal {
                    st.name = v;
                }
                iced::Task::none()
            }
            TermiusModalMsg::GroupParent(p) => {
                if let Some(TermiusModal::Group(st)) = &mut self.termius_modal {
                    st.parent_id = p;
                }
                iced::Task::none()
            }
            TermiusModalMsg::GroupIdentity(i) => {
                if let Some(TermiusModal::Group(st)) = &mut self.termius_modal {
                    st.identity_id = i;
                }
                iced::Task::none()
            }
            TermiusModalMsg::GroupJumpHost(j) => {
                if let Some(TermiusModal::Group(st)) = &mut self.termius_modal {
                    st.proxy_jump_id = j;
                }
                iced::Task::none()
            }
            TermiusModalMsg::GroupDefaultPort(p) => {
                if let Some(TermiusModal::Group(st)) = &mut self.termius_modal {
                    st.default_port = p;
                }
                iced::Task::none()
            }
            TermiusModalMsg::GroupTags(t) => {
                if let Some(TermiusModal::Group(st)) = &mut self.termius_modal {
                    st.tags = t;
                }
                iced::Task::none()
            }
            TermiusModalMsg::GroupSave => {
                let Some(TermiusModal::Group(st)) = &mut self.termius_modal else {
                    return iced::Task::none();
                };
                let default_port = st
                    .default_port
                    .trim()
                    .parse::<u16>()
                    .ok()
                    .filter(|&p| p > 0);
                let tags = st
                    .tags
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(ToString::to_string)
                    .collect();
                let grp = HostGroup {
                    id: st.editing_id.clone().unwrap_or_default(),
                    name: st.name.clone(),
                    parent_id: st.parent_id.clone(),
                    identity_id: st.identity_id.clone(),
                    proxy_jump_id: st.proxy_jump_id.clone(),
                    default_port,
                    tags,
                };
                let editing = st.editing_id.clone();
                let res = self.with_vault(|v| {
                    if let Some(id) = editing {
                        v.edit_group(&id, grp)
                    } else {
                        v.add_group(grp).map(|_| ())
                    }
                });
                match res {
                    Ok(()) => {
                        self.termius_modal = None;
                        self.termius_modal_window = None;
                        self.reload_termius_caches();
                    }
                    Err(e) => {
                        if let Some(TermiusModal::Group(st)) = &mut self.termius_modal {
                            st.error = Some(e.to_string());
                        }
                    }
                }
                iced::Task::none()
            }
            TermiusModalMsg::GroupDelete(gid) => {
                let _ = self.with_vault(|v| v.delete_group(&gid));
                if self.selected_group_filter.as_deref() == Some(gid.as_str()) {
                    self.selected_group_filter = None;
                }
                self.termius_modal = None;
                self.termius_modal_window = None;
                self.reload_termius_caches();
                iced::Task::none()
            }

            // ── Host Importer (~/.ssh/config, PuTTY, CSV, Cloud) ──────────────
            TermiusModalMsg::ImportTab(tab) => {
                if let Some(TermiusModal::ImportHosts(st)) = &mut self.termius_modal {
                    st.tab = tab;
                    st.error = None;
                    st.status = None;
                }
                iced::Task::none()
            }
            TermiusModalMsg::ImportSshPath(p) => {
                if let Some(TermiusModal::ImportHosts(st)) = &mut self.termius_modal {
                    st.ssh_config_path = p;
                }
                iced::Task::none()
            }
            TermiusModalMsg::ImportBrowseSshPath => iced::Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .set_title("Select OpenSSH config file")
                        .pick_file()
                        .await
                        .map(|h| h.path().display().to_string())
                },
                Message::ImportHostsSshFilePicked,
            ),
            TermiusModalMsg::ImportParseSshConfig => {
                if let Some(TermiusModal::ImportHosts(st)) = &mut self.termius_modal {
                    match std::fs::read_to_string(st.ssh_config_path.trim()) {
                        Ok(txt) => {
                            let parsed = parse_ssh_config(&txt);
                            st.selected = vec![true; parsed.len()];
                            st.status =
                                Some(format!("Parsed {} OpenSSH host block(s).", parsed.len()));
                            st.error = None;
                            st.preview = parsed;
                        }
                        Err(e) => {
                            st.error = Some(format!("Failed to read SSH config: {e}"));
                        }
                    }
                }
                iced::Task::none()
            }
            TermiusModalMsg::ImportExportSshConfig => {
                let text_out = export_ssh_config(&self.servers);
                iced::Task::perform(
                    async move {
                        let Some(h) = rfd::AsyncFileDialog::new()
                            .set_title("Export Glyph Servers to OpenSSH Config")
                            .set_file_name("glyph_ssh_config")
                            .save_file()
                            .await
                        else {
                            return Ok(None);
                        };
                        let p = h.path().to_path_buf();
                        tokio::fs::write(&p, text_out.as_bytes())
                            .await
                            .map_err(|e| e.to_string())?;
                        Ok(Some(p.display().to_string()))
                    },
                    Message::ExportSshConfigDone,
                )
            }
            TermiusModalMsg::ImportBrowsePuttyCsv => iced::Task::perform(
                async {
                    let Some(h) = rfd::AsyncFileDialog::new()
                        .set_title("Select PuTTY .reg or CSV hosts file")
                        .add_filter("Hosts Import", &["reg", "csv", "txt"])
                        .pick_file()
                        .await
                    else {
                        return Ok(None);
                    };
                    let path = h.path().display().to_string();
                    let body = tokio::fs::read_to_string(h.path())
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(Some((path, body)))
                },
                Message::ImportPuttyFileLoaded,
            ),
            TermiusModalMsg::ImportPuttyText(txt) => {
                if let Some(TermiusModal::ImportHosts(st)) = &mut self.termius_modal {
                    st.putty_text = txt;
                }
                iced::Task::none()
            }
            TermiusModalMsg::ImportParsePuttyCsv => {
                if let Some(TermiusModal::ImportHosts(st)) = &mut self.termius_modal {
                    let raw = st.putty_text.as_str();
                    let parsed = if raw.contains("\\Sessions\\")
                        || raw.starts_with("Windows Registry Editor")
                    {
                        parse_putty_reg(raw)
                    } else {
                        parse_hosts_csv(raw)
                    };
                    st.selected = vec![true; parsed.len()];
                    st.status = Some(format!("Parsed {} host(s).", parsed.len()));
                    st.error = None;
                    st.preview = parsed;
                }
                iced::Task::none()
            }
            TermiusModalMsg::ImportCloudProvider(p) => {
                if let Some(TermiusModal::ImportHosts(st)) = &mut self.termius_modal {
                    st.cloud_provider = p;
                }
                iced::Task::none()
            }
            TermiusModalMsg::ImportCloudToken(t) => {
                if let Some(TermiusModal::ImportHosts(st)) = &mut self.termius_modal {
                    st.cloud_token = t;
                }
                iced::Task::none()
            }
            TermiusModalMsg::ImportCloudTailnet(tn) => {
                if let Some(TermiusModal::ImportHosts(st)) = &mut self.termius_modal {
                    st.cloud_tailnet = tn;
                }
                iced::Task::none()
            }
            TermiusModalMsg::ImportFetchCloud => {
                let Some(TermiusModal::ImportHosts(st)) = &mut self.termius_modal else {
                    return iced::Task::none();
                };
                let provider = st.cloud_provider;
                let token = st.cloud_token.clone();
                let tailnet =
                    (!st.cloud_tailnet.trim().is_empty()).then(|| st.cloud_tailnet.clone());
                st.loading = true;
                st.error = None;
                st.status = None;
                iced::Task::perform(
                    async move {
                        fetch_cloud_hosts(provider, &token, tailnet.as_deref())
                            .await
                            .map_err(|e| e.to_string())
                    },
                    Message::CloudHostsFetched,
                )
            }
            TermiusModalMsg::ImportTargetGroup(g) => {
                if let Some(TermiusModal::ImportHosts(st)) = &mut self.termius_modal {
                    st.target_group_id = g;
                }
                iced::Task::none()
            }
            TermiusModalMsg::ImportToggleRow(idx, val) => {
                if let Some(TermiusModal::ImportHosts(st)) = &mut self.termius_modal {
                    if let Some(slot) = st.selected.get_mut(idx) {
                        *slot = val;
                    }
                }
                iced::Task::none()
            }
            TermiusModalMsg::ImportSelectAll(all) => {
                if let Some(TermiusModal::ImportHosts(st)) = &mut self.termius_modal {
                    st.selected.fill(all);
                }
                iced::Task::none()
            }
            TermiusModalMsg::ImportCommit => {
                let Some(TermiusModal::ImportHosts(st)) = &mut self.termius_modal else {
                    return iced::Task::none();
                };
                let mut picked = Vec::new();
                for (idx, mut h) in st.preview.iter().cloned().enumerate() {
                    if st.selected.get(idx).copied().unwrap_or(false) {
                        if let Some(gid) = &st.target_group_id {
                            h.group_id = Some(gid.clone());
                        }
                        picked.push(h);
                    }
                }
                let res = self.with_vault(|v| {
                    let mut count = 0usize;
                    for item in picked {
                        v.add_server(item)?;
                        count += 1;
                    }
                    Ok::<usize, anyhow::Error>(count)
                });
                match res {
                    Ok(n) => {
                        self.refresh_vault_cache();
                        self.reload_termius_caches();
                        if let Some(TermiusModal::ImportHosts(st)) = &mut self.termius_modal {
                            st.preview.clear();
                            st.selected.clear();
                            st.error = None;
                            st.status = Some(format!("Imported {n} host(s) into Vault."));
                        }
                    }
                    Err(e) => {
                        if let Some(TermiusModal::ImportHosts(st)) = &mut self.termius_modal {
                            st.error = Some(e.to_string());
                        }
                    }
                }
                iced::Task::none()
            }

            // ── Parameterized Snippet Execution ───────────────────────────────
            TermiusModalMsg::SnippetVarChanged(idx, v) => {
                if let Some(TermiusModal::SnippetVars(st)) = &mut self.termius_modal {
                    if let Some(pair) = st.vars.get_mut(idx) {
                        pair.1 = v;
                    }
                }
                iced::Task::none()
            }
            TermiusModalMsg::SnippetVarRun => {
                let Some(TermiusModal::SnippetVars(st)) = self.termius_modal.take() else {
                    return iced::Task::none();
                };
                self.termius_modal_window = None;
                let map: HashMap<String, String> = st.vars.into_iter().collect();
                let rendered = render_template_cmd(&st.template, &map);
                self.execute_snippet_now(st.window_id, st.snippet_id, rendered);
                iced::Task::none()
            }

            // ── Parallel Multi-Host Group Execution ───────────────────────────
            TermiusModalMsg::GroupRunCommandChanged(cmd) => {
                if let Some(TermiusModal::GroupRun(st)) = &mut self.termius_modal {
                    st.command = cmd;
                }
                iced::Task::none()
            }
            TermiusModalMsg::GroupRunGroupChanged(gid) => {
                if let Some(TermiusModal::GroupRun(st)) = &mut self.termius_modal {
                    st.selected_group_id = gid;
                }
                iced::Task::none()
            }
            TermiusModalMsg::GroupRunExecute => {
                let (cmd, group_filter) = match &self.termius_modal {
                    Some(TermiusModal::GroupRun(st)) => {
                        (st.command.trim().to_string(), st.selected_group_id.clone())
                    }
                    _ => return iced::Task::none(),
                };
                if cmd.is_empty() {
                    if let Some(TermiusModal::GroupRun(st)) = &mut self.termius_modal {
                        st.error = Some("Enter a shell command to run.".to_string());
                    }
                    return iced::Task::none();
                }

                let targets = self.with_vault(|v| {
                    let mut list = Vec::new();
                    let known = v.known_hosts_map();
                    for s in &self.servers {
                        let matches = match &group_filter {
                            None => true,
                            Some(gid) => v.is_server_in_group_subtree(s, gid),
                        };
                        if matches {
                            let cfg = v.get_connection_config(&s.id).ok();
                            list.push((s.id.clone(), display_server_name(s), cfg, known.clone()));
                        }
                    }
                    list
                });

                if targets.is_empty() {
                    if let Some(TermiusModal::GroupRun(st)) = &mut self.termius_modal {
                        st.error = Some("No servers match the chosen group.".to_string());
                    }
                    return iced::Task::none();
                }

                if let Some(TermiusModal::GroupRun(st)) = &mut self.termius_modal {
                    st.error = None;
                    st.running = true;
                    st.results = targets
                        .iter()
                        .map(|(sid, sname, _, _)| GroupRunTargetResult {
                            server_id: sid.clone(),
                            server_name: sname.clone(),
                            status: "Running".to_string(),
                            output: "Executing...".to_string(),
                        })
                        .collect();
                }

                for (sid, _sname, cfg_opt, known_hosts) in targets {
                    let active_sess = self
                        .server_windows
                        .values()
                        .find(|sw| sw.server.id == sid)
                        .map(|sw| sw.session.clone());
                    let cmd_c = cmd.clone();
                    tokio::spawn(async move {
                        let res = if let Some(sess) = active_sess {
                            sess.exec(&cmd_c).await.map_err(|e| e.to_string())
                        } else if let Some(cfg) = cfg_opt {
                            let (ev_tx, mut ev_rx) = mpsc::unbounded_channel::<SessionEvent>();
                            let (hk_tx, mut hk_rx) = mpsc::unbounded_channel::<HostKeyPrompt>();
                            tokio::spawn(async move { while ev_rx.recv().await.is_some() {} });
                            tokio::spawn(async move {
                                while let Some(prompt) = hk_rx.recv().await {
                                    let _ = prompt.responder.send(false);
                                }
                            });
                            match SshSession::connect(cfg, known_hosts, ev_tx, hk_tx).await {
                                Ok(outcome) => {
                                    let out = outcome
                                        .session
                                        .exec(&cmd_c)
                                        .await
                                        .map_err(|e| e.to_string());
                                    outcome.session.disconnect().await;
                                    out
                                }
                                Err(e) => Err(e.to_string()),
                            }
                        } else {
                            Err("Failed to build SSH config".to_string())
                        };
                        let _ = app_bus_sender().send(BusEvent::GroupRunHostDone {
                            server_id: sid,
                            result: res,
                        });
                    });
                }

                iced::Task::none()
            }
        }
    }

    pub(crate) fn handle_settings_termius_msg(
        &mut self,
        msg: ModalMsg,
    ) -> Option<iced::Task<Message>> {
        match msg {
            ModalMsg::LockPasscodeInput(v) => {
                if let Some(ManagerModal::Settings(st)) = &mut self.modal {
                    st.lock_new_passcode = v;
                }
                Some(iced::Task::none())
            }
            ModalMsg::LockConfirmInput(v) => {
                if let Some(ManagerModal::Settings(st)) = &mut self.modal {
                    st.lock_confirm_passcode = v;
                }
                Some(iced::Task::none())
            }
            ModalMsg::LockTimeoutInput(v) => {
                if let Some(ManagerModal::Settings(st)) = &mut self.modal {
                    st.lock_idle_mins = v;
                }
                Some(iced::Task::none())
            }
            ModalMsg::SaveAppLock => {
                let (pass, confirm, mins) = {
                    let Some(ManagerModal::Settings(st)) = &self.modal else {
                        return Some(iced::Task::none());
                    };
                    (
                        st.lock_new_passcode.clone(),
                        st.lock_confirm_passcode.clone(),
                        st.lock_idle_mins.trim().parse::<u32>().unwrap_or(15),
                    )
                };
                if pass != confirm {
                    if let Some(ManagerModal::Settings(st)) = &mut self.modal {
                        st.lock_error = Some("Passcodes do not match.".to_string());
                    }
                    return Some(iced::Task::none());
                }
                let res = self.with_vault(|v| v.set_app_lock_passcode(&pass, mins));
                match res {
                    Ok(()) => {
                        self.reload_termius_caches();
                        if let Some(ManagerModal::Settings(st)) = &mut self.modal {
                            st.lock_enabled = true;
                            st.lock_new_passcode.clear();
                            st.lock_confirm_passcode.clear();
                            st.lock_error = None;
                            st.lock_status = Some("Master passcode enabled.".to_string());
                        }
                    }
                    Err(e) => {
                        if let Some(ManagerModal::Settings(st)) = &mut self.modal {
                            st.lock_error = Some(e.to_string());
                        }
                    }
                }
                Some(iced::Task::none())
            }
            ModalMsg::UpdateLockTimeout => {
                let mins = match &self.modal {
                    Some(ManagerModal::Settings(st)) => {
                        st.lock_idle_mins.trim().parse::<u32>().unwrap_or(15)
                    }
                    _ => 15,
                };
                let res = self.with_vault(|v| v.set_app_lock_timeout(mins));
                self.reload_termius_caches();
                if let Some(ManagerModal::Settings(st)) = &mut self.modal {
                    match res {
                        Ok(()) => {
                            st.lock_error = None;
                            st.lock_status = Some(format!("Idle lock timeout set to {mins} min."));
                        }
                        Err(e) => st.lock_error = Some(e.to_string()),
                    }
                }
                Some(iced::Task::none())
            }
            ModalMsg::DisableAppLock => {
                let res = self.with_vault(|v| v.disable_app_lock());
                self.reload_termius_caches();
                if let Some(ManagerModal::Settings(st)) = &mut self.modal {
                    match res {
                        Ok(()) => {
                            st.lock_enabled = false;
                            st.lock_error = None;
                            st.lock_status = Some("Master passcode lock disabled.".to_string());
                        }
                        Err(e) => st.lock_error = Some(e.to_string()),
                    }
                }
                Some(iced::Task::none())
            }
            ModalMsg::LockNow => {
                self.modal = None;
                self.locked = true;
                self.lock_screen = LockScreenState::default();
                Some(iced::Task::none())
            }
            ModalMsg::ToggleHighlightRule(id) => {
                if let Some(ManagerModal::Settings(st)) = &mut self.modal {
                    if let Some(r) = st.highlight_rules.iter_mut().find(|r| r.id == id) {
                        r.enabled = !r.enabled;
                    }
                    let rules = st.highlight_rules.clone();
                    let _ = self.with_vault(|v| v.save_highlight_rules(rules));
                }
                self.reload_termius_caches();
                Some(iced::Task::none())
            }
            ModalMsg::NewHlName(v) => {
                if let Some(ManagerModal::Settings(st)) = &mut self.modal {
                    st.new_hl_name = v;
                }
                Some(iced::Task::none())
            }
            ModalMsg::NewHlPattern(v) => {
                if let Some(ManagerModal::Settings(st)) = &mut self.modal {
                    st.new_hl_pattern = v;
                }
                Some(iced::Task::none())
            }
            ModalMsg::NewHlColor(v) => {
                if let Some(ManagerModal::Settings(st)) = &mut self.modal {
                    st.new_hl_color = v;
                }
                Some(iced::Task::none())
            }
            ModalMsg::AddHighlightRule => {
                if let Some(ManagerModal::Settings(st)) = &mut self.modal {
                    let pat = st.new_hl_pattern.trim();
                    let name = st.new_hl_name.trim();
                    if name.is_empty() || pat.is_empty() {
                        st.hl_error = Some("Name and regex pattern are required.".to_string());
                        return Some(iced::Task::none());
                    }
                    st.highlight_rules.push(HighlightRule {
                        id: Uuid::new_v4().to_string(),
                        name: name.to_string(),
                        pattern: pat.to_string(),
                        color_hex: if st.new_hl_color.trim().is_empty() {
                            "#f87171".to_string()
                        } else {
                            st.new_hl_color.trim().to_string()
                        },
                        enabled: true,
                    });
                    st.new_hl_name.clear();
                    st.new_hl_pattern.clear();
                    st.hl_error = None;
                    let rules = st.highlight_rules.clone();
                    let _ = self.with_vault(|v| v.save_highlight_rules(rules));
                }
                self.reload_termius_caches();
                Some(iced::Task::none())
            }
            ModalMsg::DeleteHighlightRule(id) => {
                if let Some(ManagerModal::Settings(st)) = &mut self.modal {
                    st.highlight_rules.retain(|r| r.id != id);
                    let rules = st.highlight_rules.clone();
                    let _ = self.with_vault(|v| v.save_highlight_rules(rules));
                }
                self.reload_termius_caches();
                Some(iced::Task::none())
            }

            // ── Sync Tab ──────────────────────────────────────────────────────
            ModalMsg::SyncSelectBackend(b) => {
                if let Some(ManagerModal::Settings(st)) = &mut self.modal {
                    st.sync_backend = b;
                }
                Some(iced::Task::none())
            }
            ModalMsg::SyncFolderInput(v) => {
                if let Some(ManagerModal::Settings(st)) = &mut self.modal {
                    st.sync_folder_path = v;
                }
                Some(iced::Task::none())
            }
            ModalMsg::SyncBrowseFolder => Some(iced::Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .set_title("Choose Encrypted Sync Folder")
                        .pick_folder()
                        .await
                        .map(|h| h.path().display().to_string())
                },
                Message::SyncFolderPicked,
            )),
            ModalMsg::SyncWebdavUrl(v) => {
                if let Some(ManagerModal::Settings(st)) = &mut self.modal {
                    st.sync_webdav_url = v;
                }
                Some(iced::Task::none())
            }
            ModalMsg::SyncWebdavUser(v) => {
                if let Some(ManagerModal::Settings(st)) = &mut self.modal {
                    st.sync_webdav_user = v;
                }
                Some(iced::Task::none())
            }
            ModalMsg::SyncWebdavPass(v) => {
                if let Some(ManagerModal::Settings(st)) = &mut self.modal {
                    st.sync_webdav_pass = v;
                }
                Some(iced::Task::none())
            }
            ModalMsg::SyncPassphrase(v) => {
                if let Some(ManagerModal::Settings(st)) = &mut self.modal {
                    st.sync_passphrase = v;
                }
                Some(iced::Task::none())
            }
            ModalMsg::SyncSaveConfig => {
                self.save_sync_from_modal();
                Some(iced::Task::none())
            }
            ModalMsg::SyncRunNow => {
                self.save_sync_from_modal();
                let Some(ManagerModal::Settings(st)) = &mut self.modal else {
                    return Some(iced::Task::none());
                };
                if st.sync_passphrase.trim().is_empty() {
                    st.sync_error = Some("Enter your Zero-Knowledge E2EE passphrase.".to_string());
                    return Some(iced::Task::none());
                }
                let pass = st.sync_passphrase.clone();
                let webdav_pw =
                    (!st.sync_webdav_pass.is_empty()).then(|| st.sync_webdav_pass.clone());
                st.sync_busy = true;
                st.sync_error = None;
                st.sync_status = None;
                let vault = Arc::clone(&self.vault);
                Some(iced::Task::perform(
                    async move {
                        run_e2ee_sync_locked(&vault, &pass, webdav_pw.as_deref())
                            .await
                            .map(|o| {
                                format!(
                                    "Synced to {} (+{} servers, +{} groups, +{} identities, +{} keys)",
                                    o.destination,
                                    o.stats.servers_added,
                                    o.stats.groups_added,
                                    o.stats.identities_added,
                                    o.stats.keys_added
                                )
                            })
                            .map_err(|e| e.to_string())
                    },
                    Message::SyncFinished,
                ))
            }
            _ => None,
        }
    }

    fn save_sync_from_modal(&mut self) {
        let (backend, folder, url, user, pass) = match &self.modal {
            Some(ManagerModal::Settings(st)) => (
                st.sync_backend,
                st.sync_folder_path.clone(),
                st.sync_webdav_url.clone(),
                st.sync_webdav_user.clone(),
                (!st.sync_webdav_pass.is_empty()).then(|| st.sync_webdav_pass.clone()),
            ),
            _ => return,
        };
        let res = self.with_vault(|v| v.save_sync_config(backend, folder, url, user, pass));
        if let Some(ManagerModal::Settings(st)) = &mut self.modal {
            match res {
                Ok(()) => {
                    st.sync_error = None;
                    st.sync_status = Some("Sync configuration saved.".to_string());
                }
                Err(e) => st.sync_error = Some(e.to_string()),
            }
        }
    }

    pub(crate) fn enqueue_upload_files(
        &mut self,
        wid: window::Id,
        remote_dir: String,
        paths: Vec<PathBuf>,
    ) {
        let Some(srv) = self.server_windows.get_mut(&wid) else {
            return;
        };
        let sess = srv.session.clone();
        srv.sftp.show_queue = true;

        for p in paths {
            let tid = Uuid::new_v4().to_string();
            let cancel = CancellationToken::new();
            let name = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file")
                .to_string();
            let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            let remote_path = join_remote_path(&remote_dir, &name);

            srv.sftp.transfer_queue.push(QueuedTransfer {
                id: tid.clone(),
                kind: TransferKind::Upload,
                filename: name.clone(),
                subtitle: format!("-> {remote_path}"),
                transferred: 0,
                total: size.max(1),
                status: TransferStatus::Active,
            });
            self.transfer_cancels.insert(tid.clone(), cancel.clone());

            let sess_c = sess.clone();
            tokio::spawn(async move {
                let bus = app_bus_sender();
                let (ptx, mut prx) = mpsc::unbounded_channel();
                let bus_p = bus.clone();
                tokio::spawn(async move {
                    while let Some(pr) = prx.recv().await {
                        let _ = bus_p.send(BusEvent::SftpProgress(wid, pr));
                    }
                });
                let res = sess_c
                    .sftp_upload_file_with_cancel(
                        &p,
                        &remote_path,
                        tid.clone(),
                        Some(ptx),
                        Some(cancel),
                    )
                    .await
                    .map(|n| format!("Uploaded {name} ({n} bytes)"))
                    .map_err(|e| e.to_string());
                let _ = bus.send(BusEvent::SftpTransferFinished(wid, tid, res));
            });
        }
    }

    pub(crate) fn enqueue_download_file(
        &mut self,
        wid: window::Id,
        remote_path: String,
        local_path: PathBuf,
    ) {
        let Some(srv) = self.server_windows.get_mut(&wid) else {
            return;
        };
        let sess = srv.session.clone();
        srv.sftp.show_queue = true;
        let tid = Uuid::new_v4().to_string();
        let cancel = CancellationToken::new();
        let name = PathBuf::from(&remote_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("download")
            .to_string();
        let total = srv
            .sftp
            .entries
            .iter()
            .find(|e| e.path == remote_path)
            .map(|e| e.size)
            .unwrap_or(0);

        srv.sftp.transfer_queue.push(QueuedTransfer {
            id: tid.clone(),
            kind: TransferKind::Download,
            filename: name.clone(),
            subtitle: format!("-> {}", local_path.display()),
            transferred: 0,
            total: total.max(1),
            status: TransferStatus::Active,
        });
        self.transfer_cancels.insert(tid.clone(), cancel.clone());

        tokio::spawn(async move {
            let bus = app_bus_sender();
            let (ptx, mut prx) = mpsc::unbounded_channel();
            let bus_p = bus.clone();
            tokio::spawn(async move {
                while let Some(pr) = prx.recv().await {
                    let _ = bus_p.send(BusEvent::SftpProgress(wid, pr));
                }
            });
            let res = sess
                .sftp_download_file_with_cancel(
                    &remote_path,
                    &local_path,
                    tid.clone(),
                    Some(ptx),
                    Some(cancel),
                )
                .await
                .map(|n| format!("Downloaded {name} ({n} bytes)"))
                .map_err(|e| e.to_string());
            let _ = bus.send(BusEvent::SftpTransferFinished(wid, tid, res));
        });
    }

    pub(crate) fn enqueue_remote_copy(
        &mut self,
        owner_wid: window::Id,
        src_sess: SshSession,
        src_path: String,
        dst_sess: SshSession,
        dst_path: String,
        filename: String,
    ) {
        let Some(srv) = self.server_windows.get_mut(&owner_wid) else {
            return;
        };
        srv.sftp.show_queue = true;
        let tid = Uuid::new_v4().to_string();
        let cancel = CancellationToken::new();
        srv.sftp.transfer_queue.push(QueuedTransfer {
            id: tid.clone(),
            kind: TransferKind::RemoteCopy,
            filename: filename.clone(),
            subtitle: format!("-> {dst_path}"),
            transferred: 0,
            total: 1,
            status: TransferStatus::Active,
        });
        self.transfer_cancels.insert(tid.clone(), cancel.clone());

        tokio::spawn(async move {
            let bus = app_bus_sender();
            let (ptx, mut prx) = mpsc::unbounded_channel();
            let bus_p = bus.clone();
            tokio::spawn(async move {
                while let Some(pr) = prx.recv().await {
                    let _ = bus_p.send(BusEvent::SftpProgress(owner_wid, pr));
                }
            });
            let res = sftp_transfer_between_sessions(
                &src_sess,
                &src_path,
                &dst_sess,
                &dst_path,
                tid.clone(),
                Some(ptx),
                Some(cancel),
            )
            .await
            .map(|n| format!("Copied {filename} ({n} bytes) between servers"))
            .map_err(|e| e.to_string());
            let _ = bus.send(BusEvent::SftpTransferFinished(owner_wid, tid, res));
        });
    }

    pub(crate) fn handle_sftp_termius_msg(&mut self, wid: window::Id, msg: SftpPageMsg) -> bool {
        match msg {
            SftpPageMsg::ToggleDualPane => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    if srv.sftp.dual_pane == DualPaneMode::Off {
                        srv.sftp.dual_pane = DualPaneMode::LocalFs;
                        if let Ok((p, list)) = list_local_dir(&srv.sftp.local_path) {
                            srv.sftp.local_path = p;
                            srv.sftp.local_entries = list;
                            srv.sftp.local_error = None;
                        }
                    } else {
                        srv.sftp.dual_pane = DualPaneMode::Off;
                    }
                }
                true
            }
            SftpPageMsg::SelectRightPane(opt) => {
                match opt.window_id {
                    None => {
                        if let Some(srv) = self.server_windows.get_mut(&wid) {
                            srv.sftp.dual_pane = DualPaneMode::LocalFs;
                            match list_local_dir(&srv.sftp.local_path) {
                                Ok((p, list)) => {
                                    srv.sftp.local_path = p;
                                    srv.sftp.local_entries = list;
                                    srv.sftp.local_error = None;
                                }
                                Err(e) => srv.sftp.local_error = Some(e),
                            }
                        }
                    }
                    Some(peer_wid) => {
                        let peer_sess = self
                            .server_windows
                            .get(&peer_wid)
                            .map(|s| s.session.clone());
                        if let (Some(srv), Some(psess)) =
                            (self.server_windows.get_mut(&wid), peer_sess)
                        {
                            srv.sftp.dual_pane = DualPaneMode::RemoteServer(peer_wid);
                            srv.sftp.remote_peer_loading = true;
                            spawn_sftp_readdir_peer(wid, psess, srv.sftp.remote_peer_path.clone());
                        }
                    }
                }
                true
            }
            SftpPageMsg::LocalGoUp => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    let target = local_parent_path(&srv.sftp.local_path);
                    match list_local_dir(&target) {
                        Ok((p, list)) => {
                            srv.sftp.local_path = p;
                            srv.sftp.local_entries = list;
                            srv.sftp.local_error = None;
                        }
                        Err(e) => srv.sftp.local_error = Some(e),
                    }
                }
                true
            }
            SftpPageMsg::LocalNavigate(path) => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    match list_local_dir(&path) {
                        Ok((p, list)) => {
                            srv.sftp.local_path = p;
                            srv.sftp.local_entries = list;
                            srv.sftp.local_error = None;
                        }
                        Err(e) => srv.sftp.local_error = Some(e),
                    }
                }
                true
            }
            SftpPageMsg::LocalRefresh => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    let cur = srv.sftp.local_path.clone();
                    match list_local_dir(&cur) {
                        Ok((p, list)) => {
                            srv.sftp.local_path = p;
                            srv.sftp.local_entries = list;
                            srv.sftp.local_error = None;
                        }
                        Err(e) => srv.sftp.local_error = Some(e),
                    }
                }
                true
            }
            SftpPageMsg::LocalUploadFile(path) => {
                if let Some(rdir) = self
                    .server_windows
                    .get(&wid)
                    .map(|s| s.sftp.current_path.clone())
                {
                    self.enqueue_upload_files(wid, rdir, vec![path]);
                }
                true
            }
            SftpPageMsg::DownloadToLocalPane(remote_path, name) => {
                if let Some(ldir) = self
                    .server_windows
                    .get(&wid)
                    .map(|s| s.sftp.local_path.clone())
                {
                    let local_target = PathBuf::from(ldir).join(&name);
                    self.enqueue_download_file(wid, remote_path, local_target);
                }
                true
            }
            SftpPageMsg::PeerGoUp => {
                if let Some(DualPaneMode::RemoteServer(peer_wid)) =
                    self.server_windows.get(&wid).map(|s| s.sftp.dual_pane)
                {
                    let peer_sess = self
                        .server_windows
                        .get(&peer_wid)
                        .map(|s| s.session.clone());
                    if let (Some(srv), Some(ps)) = (self.server_windows.get_mut(&wid), peer_sess) {
                        let parent = crate::app::remote_parent(&srv.sftp.remote_peer_path);
                        srv.sftp.remote_peer_loading = true;
                        spawn_sftp_readdir_peer(wid, ps, parent);
                    }
                }
                true
            }
            SftpPageMsg::PeerNavigate(path) => {
                if let Some(DualPaneMode::RemoteServer(peer_wid)) =
                    self.server_windows.get(&wid).map(|s| s.sftp.dual_pane)
                {
                    let peer_sess = self
                        .server_windows
                        .get(&peer_wid)
                        .map(|s| s.session.clone());
                    if let (Some(srv), Some(ps)) = (self.server_windows.get_mut(&wid), peer_sess) {
                        srv.sftp.remote_peer_loading = true;
                        spawn_sftp_readdir_peer(wid, ps, path);
                    }
                }
                true
            }
            SftpPageMsg::PeerRefresh => {
                if let Some(DualPaneMode::RemoteServer(peer_wid)) =
                    self.server_windows.get(&wid).map(|s| s.sftp.dual_pane)
                {
                    let peer_sess = self
                        .server_windows
                        .get(&peer_wid)
                        .map(|s| s.session.clone());
                    if let (Some(srv), Some(ps)) = (self.server_windows.get_mut(&wid), peer_sess) {
                        srv.sftp.remote_peer_loading = true;
                        spawn_sftp_readdir_peer(wid, ps, srv.sftp.remote_peer_path.clone());
                    }
                }
                true
            }
            SftpPageMsg::CopyLeftToPeer(src_path, name) => {
                let info = self
                    .server_windows
                    .get(&wid)
                    .and_then(|s| match s.sftp.dual_pane {
                        DualPaneMode::RemoteServer(pwid) => Some((
                            s.session.clone(),
                            pwid,
                            join_remote_path(&s.sftp.remote_peer_path, &name),
                        )),
                        _ => None,
                    });
                if let Some((src_sess, peer_wid, dst_path)) = info {
                    if let Some(dst_sess) = self
                        .server_windows
                        .get(&peer_wid)
                        .map(|s| s.session.clone())
                    {
                        self.enqueue_remote_copy(wid, src_sess, src_path, dst_sess, dst_path, name);
                    }
                }
                true
            }
            SftpPageMsg::CopyPeerToLeft(src_path, name) => {
                let info = self
                    .server_windows
                    .get(&wid)
                    .and_then(|s| match s.sftp.dual_pane {
                        DualPaneMode::RemoteServer(pwid) => Some((
                            pwid,
                            s.session.clone(),
                            join_remote_path(&s.sftp.current_path, &name),
                        )),
                        _ => None,
                    });
                if let Some((peer_wid, dst_sess, dst_path)) = info {
                    if let Some(src_sess) = self
                        .server_windows
                        .get(&peer_wid)
                        .map(|s| s.session.clone())
                    {
                        self.enqueue_remote_copy(wid, src_sess, src_path, dst_sess, dst_path, name);
                    }
                }
                true
            }
            SftpPageMsg::ToggleQueueDrawer => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    srv.sftp.show_queue = !srv.sftp.show_queue;
                }
                true
            }
            SftpPageMsg::CancelTransfer(tid) => {
                if let Some(tok) = self.transfer_cancels.remove(&tid) {
                    tok.cancel();
                }
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    if let Some(item) = srv.sftp.transfer_queue.iter_mut().find(|q| q.id == tid) {
                        item.status = TransferStatus::Cancelled;
                    }
                }
                true
            }
            SftpPageMsg::ClearCompletedTransfers => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    srv.sftp
                        .transfer_queue
                        .retain(|q| matches!(q.status, TransferStatus::Active));
                }
                true
            }
            _ => false,
        }
    }

    pub(crate) fn handle_tunnels_termius_msg(
        &mut self,
        wid: window::Id,
        msg: TunnelsPageMsg,
    ) -> bool {
        let Some(srv) = self.server_windows.get_mut(&wid) else {
            return false;
        };
        let sess = srv.session.clone();
        let sid = srv.server.id.clone();

        match msg {
            TunnelsPageMsg::ToggleSaveProfile(v) => {
                srv.tunnels.save_as_profile = v;
                true
            }
            TunnelsPageMsg::ProfileNameChanged(v) => {
                srv.tunnels.profile_name = v;
                true
            }
            TunnelsPageMsg::ToggleProfileAutoStart(v) => {
                srv.tunnels.profile_auto_start = v;
                true
            }
            TunnelsPageMsg::StartSavedProfile(rule_id) => {
                if let Some(rule) = srv
                    .server
                    .saved_tunnels
                    .iter()
                    .find(|r| r.id == rule_id)
                    .cloned()
                {
                    srv.tunnels.busy = true;
                    srv.tunnels.error = None;
                    tokio::spawn(async move {
                        let op = match rule.direction {
                            TunnelDirection::Local => sess
                                .start_local_tunnel(
                                    rule.local_port,
                                    rule.remote_host,
                                    rule.remote_port,
                                    rule.protocol,
                                )
                                .await
                                .map(|_| ()),
                            TunnelDirection::Remote => sess
                                .start_remote_tunnel(
                                    "0.0.0.0".to_string(),
                                    rule.remote_port,
                                    rule.remote_host,
                                    rule.local_port,
                                )
                                .await
                                .map(|_| ()),
                            TunnelDirection::Dynamic => sess
                                .start_dynamic_socks5_tunnel(rule.local_port)
                                .await
                                .map(|_| ()),
                        };
                        let res = match op {
                            Ok(()) => Ok(sess.list_tunnels().await),
                            Err(e) => Err(e.to_string()),
                        };
                        let _ = app_bus_sender().send(BusEvent::TunnelOpDone(wid, res));
                    });
                }
                true
            }
            TunnelsPageMsg::ToggleSavedAutoStart(rule_id) => {
                let updated = self.with_vault(|v| {
                    let _ = v.toggle_saved_tunnel_autostart(&sid, &rule_id);
                    v.get_server(&sid)
                });
                self.refresh_vault_cache();
                if let (Some(srv_mut), Some(s)) = (self.server_windows.get_mut(&wid), updated) {
                    srv_mut.server = s;
                }
                true
            }
            TunnelsPageMsg::DeleteSavedProfile(rule_id) => {
                let updated = self.with_vault(|v| {
                    let _ = v.delete_saved_tunnel(&sid, &rule_id);
                    v.get_server(&sid)
                });
                self.refresh_vault_cache();
                if let (Some(srv_mut), Some(s)) = (self.server_windows.get_mut(&wid), updated) {
                    srv_mut.server = s;
                }
                true
            }
            TunnelsPageMsg::BindSubmit => {
                let bind_port: u16 = match srv.tunnels.bind_port.trim().parse() {
                    Ok(p) => p,
                    Err(_) => {
                        srv.tunnels.error =
                            Some("Enter a valid numeric bind port (0-65535).".into());
                        return true;
                    }
                };
                let dir = srv.tunnels.direction;
                let save_profile = srv.tunnels.save_as_profile;
                let auto_start = srv.tunnels.profile_auto_start;
                let prof_name = srv.tunnels.profile_name.trim().to_string();
                let proto = srv.tunnels.protocol.clone();

                if dir == UiTunnelDirection::Dynamic {
                    srv.tunnels.busy = true;
                    srv.tunnels.error = None;
                    if save_profile {
                        let rule = SavedTunnelRule {
                            id: Uuid::new_v4().to_string(),
                            name: if prof_name.is_empty() {
                                format!("SOCKS5 :{bind_port}")
                            } else {
                                prof_name
                            },
                            direction: TunnelDirection::Dynamic,
                            local_port: bind_port,
                            remote_host: "socks5".to_string(),
                            remote_port: 0,
                            protocol: "TCP".to_string(),
                            auto_start,
                        };
                        let updated = self.with_vault(|v| {
                            let _ = v.add_saved_tunnel(&sid, rule);
                            v.get_server(&sid)
                        });
                        self.refresh_vault_cache();
                        if let (Some(srv_mut), Some(s)) =
                            (self.server_windows.get_mut(&wid), updated)
                        {
                            srv_mut.server = s;
                            srv_mut.tunnels.profile_name.clear();
                        }
                    }
                    tokio::spawn(async move {
                        let res = match sess.start_dynamic_socks5_tunnel(bind_port).await {
                            Ok(_) => Ok(sess.list_tunnels().await),
                            Err(e) => Err(e.to_string()),
                        };
                        let _ = app_bus_sender().send(BusEvent::TunnelOpDone(wid, res));
                    });
                    return true;
                }

                let target_port: u16 = match srv.tunnels.target_port.trim().parse() {
                    Ok(p) if p > 0 => p,
                    _ => {
                        srv.tunnels.error = Some("Enter a valid non-zero destination port.".into());
                        return true;
                    }
                };
                let target_host = srv.tunnels.target_host.trim().to_string();
                srv.tunnels.busy = true;
                srv.tunnels.error = None;

                if save_profile {
                    let rule_dir = if dir == UiTunnelDirection::Local {
                        TunnelDirection::Local
                    } else {
                        TunnelDirection::Remote
                    };
                    let rule = SavedTunnelRule {
                        id: Uuid::new_v4().to_string(),
                        name: if prof_name.is_empty() {
                            format!("{bind_port}->{target_host}:{target_port}")
                        } else {
                            prof_name
                        },
                        direction: rule_dir,
                        local_port: if dir == UiTunnelDirection::Local {
                            bind_port
                        } else {
                            target_port
                        },
                        remote_host: target_host.clone(),
                        remote_port: if dir == UiTunnelDirection::Local {
                            target_port
                        } else {
                            bind_port
                        },
                        protocol: proto.clone(),
                        auto_start,
                    };
                    let updated = self.with_vault(|v| {
                        let _ = v.add_saved_tunnel(&sid, rule);
                        v.get_server(&sid)
                    });
                    self.refresh_vault_cache();
                    if let (Some(srv_mut), Some(s)) = (self.server_windows.get_mut(&wid), updated) {
                        srv_mut.server = s;
                        srv_mut.tunnels.profile_name.clear();
                    }
                }

                tokio::spawn(async move {
                    let op = match dir {
                        UiTunnelDirection::Local => sess
                            .start_local_tunnel(bind_port, target_host, target_port, proto)
                            .await
                            .map(|_| ()),
                        UiTunnelDirection::Remote => sess
                            .start_remote_tunnel(
                                "0.0.0.0".to_string(),
                                bind_port,
                                target_host,
                                target_port,
                            )
                            .await
                            .map(|_| ()),
                        UiTunnelDirection::Dynamic => unreachable!(),
                    };
                    let res = match op {
                        Ok(()) => Ok(sess.list_tunnels().await),
                        Err(e) => Err(e.to_string()),
                    };
                    let _ = app_bus_sender().send(BusEvent::TunnelOpDone(wid, res));
                });
                true
            }
            _ => false,
        }
    }

    pub(crate) fn handle_commands_termius_msg(
        &mut self,
        wid: window::Id,
        msg: CommandsPageMsg,
    ) -> bool {
        match msg {
            CommandsPageMsg::SwitchSubView(sv) => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    srv.commands.sub_view = sv;
                    if sv == CommandsSubView::Packages
                        && srv.commands.selected_package_id.is_empty()
                    {
                        if let Some(first) = self.snippet_packages.first() {
                            srv.commands.selected_package_id = first.id.clone();
                        }
                    }
                }
                true
            }
            CommandsPageMsg::Run(id, title, cmd) => {
                self.trigger_snippet_or_prompt(wid, Some(id), title, cmd);
                true
            }
            CommandsPageMsg::RunPackageSnippet(title, cmd) => {
                self.trigger_snippet_or_prompt(wid, None, title, cmd);
                true
            }
            CommandsPageMsg::RunOnGroup(title, cmd) => {
                let default_gid = self
                    .server_windows
                    .get(&wid)
                    .and_then(|s| s.server.group_id.clone());
                self.termius_modal_window = Some(wid);
                self.termius_modal = Some(TermiusModal::GroupRun(GroupRunModalState {
                    title,
                    command: cmd,
                    selected_group_id: default_gid,
                    results: Vec::new(),
                    running: false,
                    error: None,
                }));
                true
            }
            CommandsPageMsg::SelectPackage(pid) => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    srv.commands.selected_package_id = pid;
                }
                true
            }
            CommandsPageMsg::ToggleNewPackageForm => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    srv.commands.show_new_pkg = !srv.commands.show_new_pkg;
                }
                true
            }
            CommandsPageMsg::NewPackageNameInput(v) => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    srv.commands.new_pkg_name = v;
                }
                true
            }
            CommandsPageMsg::NewPackageDescInput(v) => {
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    srv.commands.new_pkg_desc = v;
                }
                true
            }
            CommandsPageMsg::AddPackageSubmit => {
                let (name, desc) = match self.server_windows.get(&wid) {
                    Some(s) => (
                        s.commands.new_pkg_name.clone(),
                        s.commands.new_pkg_desc.clone(),
                    ),
                    None => return true,
                };
                let res = self.with_vault(|v| v.add_snippet_package(&name, &desc));
                match res {
                    Ok(pid) => {
                        self.reload_termius_caches();
                        if let Some(srv) = self.server_windows.get_mut(&wid) {
                            srv.commands.selected_package_id = pid;
                            srv.commands.new_pkg_name.clear();
                            srv.commands.new_pkg_desc.clear();
                            srv.commands.show_new_pkg = false;
                            srv.commands.error = None;
                        }
                    }
                    Err(e) => {
                        if let Some(srv) = self.server_windows.get_mut(&wid) {
                            srv.commands.error = Some(e.to_string());
                        }
                    }
                }
                true
            }
            CommandsPageMsg::DeletePackage(pid) => {
                let _ = self.with_vault(|v| v.delete_snippet_package(&pid));
                self.reload_termius_caches();
                if let Some(srv) = self.server_windows.get_mut(&wid) {
                    srv.commands.selected_package_id = self
                        .snippet_packages
                        .first()
                        .map(|p| p.id.clone())
                        .unwrap_or_default();
                }
                true
            }
            CommandsPageMsg::AddPackageSnippetSubmit => {
                let (pid, name, cmd) = match self.server_windows.get(&wid) {
                    Some(s) => {
                        let pid = if s.commands.selected_package_id.is_empty() {
                            self.snippet_packages
                                .first()
                                .map(|p| p.id.clone())
                                .unwrap_or_default()
                        } else {
                            s.commands.selected_package_id.clone()
                        };
                        (pid, s.commands.new_name.clone(), s.commands.new_cmd.clone())
                    }
                    None => return true,
                };
                if name.trim().is_empty() || cmd.trim().is_empty() {
                    if let Some(srv) = self.server_windows.get_mut(&wid) {
                        srv.commands.error =
                            Some("Enter both a Snippet Name and Shell Command.".into());
                    }
                    return true;
                }
                let res = self.with_vault(|v| v.add_package_snippet(&pid, &name, &cmd));
                match res {
                    Ok(_) => {
                        self.reload_termius_caches();
                        if let Some(srv) = self.server_windows.get_mut(&wid) {
                            srv.commands.new_name.clear();
                            srv.commands.new_cmd.clear();
                            srv.commands.error = None;
                        }
                    }
                    Err(e) => {
                        if let Some(srv) = self.server_windows.get_mut(&wid) {
                            srv.commands.error = Some(e.to_string());
                        }
                    }
                }
                true
            }
            CommandsPageMsg::DeletePackageSnippet(pid, sid) => {
                let _ = self.with_vault(|v| v.remove_package_snippet(&pid, sid));
                self.reload_termius_caches();
                true
            }
            _ => false,
        }
    }
}
