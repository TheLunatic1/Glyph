use crate::theme::{
    brand_button, danger_button, danger_ghost_button, dark_card, dark_input, dark_pick_list,
    dark_scrollable, ghost_icon_button, modal_backdrop, modal_card, pill_badge, rgba,
    secondary_button, APP_VERSION, BRAND_300, BRAND_400, BRAND_500, DARK_600, DARK_700, DARK_800,
    DARK_900, FONT_MONO, FONT_MONO_BOLD, GREEN_400, GREEN_500, ORANGE_400, PURPLE_400, RED_400,
    RED_500, TEXT_DIM, TEXT_LIGHT, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY, YELLOW_400,
    YELLOW_500,
};
use crate::ui::icons;
use crate::ui::os_logo::os_logo;
use glyph_core::mcp_config::{
    build_config_snippet, resolve_mcp_binary_path, scan_mcp_clients, McpClientId, McpClientInfo,
    McpClientStatus, McpConfigFormat,
};
use glyph_core::models::{
    KnownHostEntry, ServerEntry, ServerSummary, SshKeyAlgorithm, SshKeySummary,
};
use glyph_core::updater::ReleaseInfo;
use iced::widget::{
    button, checkbox, column, container, image, mouse_area, pick_list, row, scrollable, space,
    stack, text, text_input,
};
use iced::{alignment, Background, Border, Color, Element, Length};
use qrcode::{Color as QrColor, QrCode};
use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerPickOption {
    pub id: String,
    pub label: String,
}

impl fmt::Display for ServerPickOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label)
    }
}

#[derive(Debug, Clone)]
pub struct DeleteConfirmState {
    pub server_id: String,
    pub server_name: String,
    pub host: String,
}

#[derive(Debug, Clone)]
pub struct ExportModalState {
    pub selected_ids: BTreeSet<String>,
    pub password: String,
    pub show_password: bool,
    pub error: Option<String>,
    pub status: Option<String>,
    pub busy: bool,
}

impl ExportModalState {
    pub fn new(servers: &[ServerSummary]) -> Self {
        Self {
            selected_ids: servers.iter().map(|s| s.id.clone()).collect(),
            password: String::new(),
            show_password: false,
            error: None,
            status: None,
            busy: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ImportStep {
    UnlockFile,
    SelectServers(Vec<ServerEntry>),
}

#[derive(Debug, Clone)]
pub struct ImportModalState {
    pub file_path: Option<String>,
    pub encrypted_content: Option<String>,
    pub password: String,
    pub show_password: bool,
    pub step: ImportStep,
    pub selected_indices: BTreeSet<usize>,
    pub error: Option<String>,
    pub busy: bool,
}

impl Default for ImportModalState {
    fn default() -> Self {
        Self {
            file_path: None,
            encrypted_content: None,
            password: String::new(),
            show_password: false,
            step: ImportStep::UnlockFile,
            selected_indices: BTreeSet::new(),
            error: None,
            busy: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SshKeysModalState {
    pub show_generate: bool,
    pub show_import: bool,
    pub gen_name: String,
    pub gen_algo: SshKeyAlgorithm,
    pub gen_passphrase: String,
    pub show_gen_pass: bool,
    pub imp_name: String,
    pub imp_path: Option<String>,
    pub imp_pem: Option<String>,
    pub imp_passphrase: String,
    pub show_imp_pass: bool,
    pub deploy_key_id: Option<String>,
    pub deploy_server_id: Option<String>,
    pub copied_key_id: Option<String>,
    pub busy: bool,
    pub error: Option<String>,
    pub status: Option<String>,
}

impl Default for SshKeysModalState {
    fn default() -> Self {
        Self {
            show_generate: false,
            show_import: false,
            gen_name: String::new(),
            gen_algo: SshKeyAlgorithm::Ed25519,
            gen_passphrase: String::new(),
            show_gen_pass: false,
            imp_name: String::new(),
            imp_path: None,
            imp_pem: None,
            imp_passphrase: String::new(),
            show_imp_pass: false,
            deploy_key_id: None,
            deploy_server_id: None,
            copied_key_id: None,
            busy: false,
            error: None,
            status: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HostKeyPromptUi {
    pub prompt_id: u64,
    pub host: String,
    pub port: u16,
    pub algorithm: String,
    pub fingerprint_sha256: String,
    pub mismatch_expected: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConnectingModalState {
    pub server_id: String,
    pub server_name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub os: Option<String>,
    pub elapsed_ms: u64,
    pub logs: Vec<String>,
    pub host_key_prompt: Option<HostKeyPromptUi>,
    pub error: Option<String>,
    pub blink_on: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    AiAgents,
    Security,
    Mobile,
    Updates,
}

#[derive(Debug, Clone)]
pub struct SettingsModalState {
    pub active_tab: SettingsTab,
    pub encryption_available: bool,
    pub data_dir: String,
    pub api_port: u16,
    pub mcp_token_path: String,
    pub mcp_bin_path: String,
    pub clients: Vec<McpClientInfo>,
    pub expanded_snippet: Option<McpClientId>,
    pub copied_snippet: Option<McpClientId>,
    pub show_custom: bool,
    pub custom_path: String,
    pub custom_format: McpConfigFormat,
    pub custom_key: String,
    pub mcp_status: Option<String>,
    pub mcp_error: Option<String>,
    pub checking_update: bool,
    pub latest_release: Option<ReleaseInfo>,
    pub is_newer_available: bool,
    pub update_error: Option<String>,
}

impl SettingsModalState {
    pub fn new(encryption_available: bool, data_dir: String) -> Self {
        let mcp_bin_path = resolve_mcp_binary_path().display().to_string();
        let api_port = crate::app::effective_api_port();
        let mcp_token_path = crate::app::effective_mcp_token_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "~/.glyph/mcp_token".to_string());
        Self {
            active_tab: SettingsTab::AiAgents,
            encryption_available,
            data_dir,
            api_port,
            mcp_token_path,
            mcp_bin_path,
            clients: scan_mcp_clients(),
            expanded_snippet: None,
            copied_snippet: None,
            show_custom: false,
            custom_path: String::new(),
            custom_format: McpConfigFormat::McpServers,
            custom_key: "mcpServers".to_string(),
            mcp_status: None,
            mcp_error: None,
            checking_update: false,
            latest_release: None,
            is_newer_available: false,
            update_error: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobileQrMode {
    DirectApk,
    GithubPage,
}

#[derive(Debug, Clone)]
pub struct MobileModalState {
    pub qr_mode: MobileQrMode,
    pub loading: bool,
    pub latest_version: Option<String>,
    pub direct_apk_url: Option<String>,
    pub page_url: String,
    pub qr_url: String,
    pub qr_handle: Option<image::Handle>,
    pub copied: bool,
    pub error: Option<String>,
}

impl Default for MobileModalState {
    fn default() -> Self {
        let default_url = "https://github.com/TheLunatic1/glyph-app/releases/latest".to_string();
        let qr_handle = generate_qr_handle(&default_url);
        Self {
            qr_mode: MobileQrMode::DirectApk,
            loading: true,
            latest_version: None,
            direct_apk_url: None,
            page_url: default_url.clone(),
            qr_url: default_url,
            qr_handle,
            copied: false,
            error: None,
        }
    }
}

impl MobileModalState {
    pub fn active_url(&self) -> &str {
        match self.qr_mode {
            MobileQrMode::DirectApk => self
                .direct_apk_url
                .as_deref()
                .unwrap_or(self.page_url.as_str()),
            MobileQrMode::GithubPage => self.page_url.as_str(),
        }
    }

    pub fn refresh_qr(&mut self) {
        let target = self.active_url().to_string();
        self.qr_handle = generate_qr_handle(&target);
        self.qr_url = target;
    }
}

pub fn generate_qr_handle(url: &str) -> Option<image::Handle> {
    let code = QrCode::new(url.as_bytes()).ok()?;
    let w = code.width();
    let colors = code.into_colors();
    let quiet = 4usize;
    let total = w + quiet * 2;
    let scale = (224 / total).max(3);
    let px = (total * scale) as u32;
    let mut rgba = Vec::with_capacity((px * px * 4) as usize);

    for py in 0..(total * scale) {
        let my = py / scale;
        for px_x in 0..(total * scale) {
            let mx = px_x / scale;
            let is_dark = if mx >= quiet && mx < quiet + w && my >= quiet && my < quiet + w {
                colors[(my - quiet) * w + (mx - quiet)] == QrColor::Dark
            } else {
                false
            };
            if is_dark {
                // #0f172a
                rgba.extend_from_slice(&[15, 23, 42, 255]);
            } else {
                // #ffffff
                rgba.extend_from_slice(&[255, 255, 255, 255]);
            }
        }
    }

    Some(image::Handle::from_rgba(px, px, rgba))
}

#[derive(Debug, Clone)]
pub enum ManagerModal {
    DeleteConfirm(DeleteConfirmState),
    Export(ExportModalState),
    Import(ImportModalState),
    SshKeys(SshKeysModalState),
    Connecting(ConnectingModalState),
    Settings(Box<SettingsModalState>),
    Mobile(MobileModalState),
}

#[derive(Debug, Clone)]
pub enum ModalMsg {
    Close,
    Noop,

    // Delete confirmation
    ConfirmDelete(String),

    // Export
    ExportToggleServer(String, bool),
    ExportSelectAll,
    ExportDeselectAll,
    ExportPasswordChanged(String),
    ExportToggleShowPassword,
    ExportSubmit,

    // Import
    ImportBrowseFile,
    ImportPasswordChanged(String),
    ImportToggleShowPassword,
    ImportDecryptSubmit,
    ImportBackToUnlock,
    ImportToggleServer(usize, bool),
    ImportSelectAll,
    ImportDeselectAll,
    ImportConfirmSubmit,

    // SSH Key Vault
    SshKeyToggleGenerate,
    SshKeyToggleImport,
    SshKeyGenName(String),
    SshKeyGenAlgo(SshKeyAlgorithm),
    SshKeyGenPass(String),
    SshKeyGenToggleShowPass,
    SshKeyGenSubmit,
    SshKeyImpName(String),
    SshKeyImpBrowseFile,
    SshKeyImpPass(String),
    SshKeyImpToggleShowPass,
    SshKeyImpSubmit,
    SshKeyCopyPublic(String),
    SshKeyToggleDeploy(String),
    SshKeySelectDeployServer(ServerPickOption),
    SshKeyDeploySubmit,
    SshKeyDelete(String),

    // Connecting + TOFU
    RespondHostKey(u64, bool),
    ConnectingRetry,

    // Settings
    SelectSettingsTab(SettingsTab),
    RemoveKnownHost(String),
    RefreshMcpClients,
    AutoInstallMcp(McpClientId),
    ToggleMcpSnippet(McpClientId),
    CopyMcpSnippet(McpClientId),
    ToggleCustomMcp,
    CustomMcpPathChanged(String),
    BrowseCustomMcpPath,
    CustomMcpFormatChanged(McpConfigFormat),
    CustomMcpKeyChanged(String),
    WriteCustomMcp,
    OpenMobileFromSettings,
    CheckUpdates,
    OpenUpdateReleaseUrl(String),

    // Mobile APK QR Modal
    SelectMobileQrMode(MobileQrMode),
    CopyMobileUrl,
    OpenMobileUrl,
}

pub fn view<'a>(
    modal: &'a ManagerModal,
    servers: &'a [ServerSummary],
    ssh_keys: &'a [SshKeySummary],
    known_hosts: &'a [KnownHostEntry],
) -> Element<'a, ModalMsg> {
    let dialog: Element<'a, ModalMsg> = match modal {
        ManagerModal::DeleteConfirm(st) => view_delete(st),
        ManagerModal::Export(st) => view_export(st, servers),
        ManagerModal::Import(st) => view_import(st),
        ManagerModal::SshKeys(st) => view_ssh_keys(st, servers, ssh_keys),
        ManagerModal::Connecting(st) => view_connecting(st),
        ManagerModal::Settings(st) => view_settings(st, known_hosts),
        ManagerModal::Mobile(st) => view_mobile(st),
    };

    let backdrop = mouse_area(
        container(space().width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(modal_backdrop),
    )
    .on_press(ModalMsg::Close);

    // Prevent clicks inside modal card from bubbling to backdrop close
    let centered_dialog = container(mouse_area(dialog).on_press(ModalMsg::Noop))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .padding(24);

    stack![backdrop, centered_dialog]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn view_delete<'a>(st: &'a DeleteConfirmState) -> Element<'a, ModalMsg> {
    let header = row![
        container(icons::alert_triangle(22.0, RED_400))
            .padding(10)
            .style(|_| container::Style {
                background: Some(Background::Color(rgba(RED_500, 0.14))),
                border: Border {
                    color: rgba(RED_500, 0.3),
                    width: 1.0,
                    radius: 10.0.into(),
                },
                ..Default::default()
            }),
        column![
            text("Delete Server")
                .size(18)
                .font(FONT_MONO_BOLD)
                .color(TEXT_PRIMARY),
            text(format!("{} ({})", st.server_name, st.host))
                .size(12)
                .color(TEXT_SECONDARY),
        ]
        .spacing(2),
        space().width(Length::Fill),
        button(icons::x_close(16.0, TEXT_SECONDARY))
            .padding(6)
            .style(ghost_icon_button)
            .on_press(ModalMsg::Close),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center);

    let body = text(format!(
        "Are you sure you want to permanently remove \"{}\" from your saved servers? Any stored credentials, secrets, and custom snippets for this server will also be removed.",
        st.server_name
    ))
    .size(13)
    .color(TEXT_LIGHT);

    let actions = row![
        space().width(Length::Fill),
        button(text("Cancel").size(13).color(TEXT_LIGHT))
            .padding([8, 16])
            .style(secondary_button)
            .on_press(ModalMsg::Close),
        button(
            row![
                icons::trash(14.0, Color::WHITE),
                text("Delete Server").size(13).font(FONT_MONO_BOLD).color(Color::WHITE),
            ]
            .spacing(6)
            .align_y(alignment::Vertical::Center)
        )
        .padding([8, 16])
        .style(danger_button)
        .on_press(ModalMsg::ConfirmDelete(st.server_id.clone())),
    ]
    .spacing(10);

    container(column![header, body, actions].spacing(18))
        .width(Length::Fixed(460.0))
        .padding(24)
        .style(modal_card)
        .into()
}

fn view_export<'a>(st: &'a ExportModalState, servers: &'a [ServerSummary]) -> Element<'a, ModalMsg> {
    let header = row![
        container(icons::download(18.0, BRAND_400))
            .padding(9)
            .style(|_| container::Style {
                background: Some(Background::Color(rgba(BRAND_500, 0.14))),
                border: Border {
                    color: rgba(BRAND_500, 0.3),
                    width: 1.0,
                    radius: 10.0.into(),
                },
                ..Default::default()
            }),
        column![
            text("Export Encrypted Backup (.glyph)")
                .size(17)
                .font(FONT_MONO_BOLD)
                .color(TEXT_PRIMARY),
            text("AES-256-GCM + scrypt · Compatible with Glyph v2 & Mobile")
                .size(11)
                .color(TEXT_SECONDARY),
        ]
        .spacing(2),
        space().width(Length::Fill),
        button(icons::x_close(16.0, TEXT_SECONDARY))
            .padding(6)
            .style(ghost_icon_button)
            .on_press(ModalMsg::Close),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center);

    let sel_bar = row![
        text(format!(
            "Select Servers ({} / {})",
            st.selected_ids.len(),
            servers.len()
        ))
        .size(12)
        .font(FONT_MONO_BOLD)
        .color(TEXT_LIGHT),
        space().width(Length::Fill),
        button(text("Select All").size(11).color(BRAND_400))
            .padding([3, 8])
            .style(ghost_icon_button)
            .on_press(ModalMsg::ExportSelectAll),
        button(text("Deselect All").size(11).color(TEXT_SECONDARY))
            .padding([3, 8])
            .style(ghost_icon_button)
            .on_press(ModalMsg::ExportDeselectAll),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center);

    let mut list_col = column![].spacing(6);
    if servers.is_empty() {
        list_col = list_col.push(
            text("No servers saved in your vault yet.")
                .size(12)
                .color(TEXT_MUTED),
        );
    } else {
        for s in servers {
            let checked = st.selected_ids.contains(&s.id);
            let sid = s.id.clone();
            let title = if s.name.is_empty() { &s.host } else { &s.name };
            let label = format!("{}  ({}@{}:{})", title, s.username, s.host, s.port);
            list_col = list_col.push(
                container(
                    checkbox(checked)
                        .label(label)
                        .size(16.0)
                        .text_size(12.0)
                        .on_toggle(move |v| ModalMsg::ExportToggleServer(sid.clone(), v)),
                )
                .width(Length::Fill)
                .padding([8, 10])
                .style(move |_| container::Style {
                    background: Some(Background::Color(if checked {
                        rgba(BRAND_500, 0.1)
                    } else {
                        DARK_800
                    })),
                    border: Border {
                        color: if checked {
                            rgba(BRAND_500, 0.35)
                        } else {
                            DARK_700
                        },
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                }),
            );
        }
    }

    let list_scroll = container(
        scrollable(list_col)
            .height(Length::Fixed(185.0))
            .style(dark_scrollable),
    )
    .padding(8)
    .style(|_| container::Style {
        background: Some(Background::Color(DARK_900)),
        border: Border {
            color: DARK_700,
            width: 1.0,
            radius: 10.0.into(),
        },
        ..Default::default()
    });

    let pw_section = column![
        text("Master Encryption Password")
            .size(12)
            .font(FONT_MONO_BOLD)
            .color(TEXT_LIGHT),
        row![
            text_input("Enter strong passphrase to lock .glyph file...", &st.password)
                .secure(!st.show_password)
                .size(13)
                .padding([9, 12])
                .style(dark_input)
                .on_input(ModalMsg::ExportPasswordChanged),
            button(if st.show_password {
                icons::eye_off(16.0, TEXT_SECONDARY)
            } else {
                icons::eye(16.0, TEXT_SECONDARY)
            })
            .padding([8, 10])
            .style(secondary_button)
            .on_press(ModalMsg::ExportToggleShowPassword),
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center),
    ]
    .spacing(6);

    let mut content = column![header, sel_bar, list_scroll, pw_section].spacing(14);
    if let Some(err) = &st.error {
        content = content.push(error_banner(err));
    }
    if let Some(ok) = &st.status {
        content = content.push(success_banner(ok));
    }

    let can_export = !st.busy && !st.selected_ids.is_empty() && !st.password.is_empty();
    let export_btn = button(
        row![
            icons::download(14.0, Color::WHITE),
            text(if st.busy { "Exporting..." } else { "Export (.glyph)" })
                .size(13)
                .font(FONT_MONO_BOLD)
                .color(Color::WHITE),
        ]
        .spacing(6)
        .align_y(alignment::Vertical::Center),
    )
    .padding([9, 16])
    .style(brand_button);
    let export_btn = if can_export {
        export_btn.on_press(ModalMsg::ExportSubmit)
    } else {
        export_btn
    };

    content = content.push(
        row![
            space().width(Length::Fill),
            button(text("Cancel").size(13).color(TEXT_LIGHT))
                .padding([9, 16])
                .style(secondary_button)
                .on_press(ModalMsg::Close),
            export_btn,
        ]
        .spacing(10),
    );

    container(content)
        .width(Length::Fixed(520.0))
        .padding(24)
        .style(modal_card)
        .into()
}

fn view_import<'a>(st: &'a ImportModalState) -> Element<'a, ModalMsg> {
    let header = row![
        container(icons::upload(18.0, BRAND_400))
            .padding(9)
            .style(|_| container::Style {
                background: Some(Background::Color(rgba(BRAND_500, 0.14))),
                border: Border {
                    color: rgba(BRAND_500, 0.3),
                    width: 1.0,
                    radius: 10.0.into(),
                },
                ..Default::default()
            }),
        column![
            text("Import Encrypted Backup (.glyph)")
                .size(17)
                .font(FONT_MONO_BOLD)
                .color(TEXT_PRIMARY),
            text("Restore servers exported from Glyph Desktop or Mobile")
                .size(11)
                .color(TEXT_SECONDARY),
        ]
        .spacing(2),
        space().width(Length::Fill),
        button(icons::x_close(16.0, TEXT_SECONDARY))
            .padding(6)
            .style(ghost_icon_button)
            .on_press(ModalMsg::Close),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center);

    let mut content = column![header].spacing(14);

    match &st.step {
        ImportStep::UnlockFile => {
            let file_label = st
                .file_path
                .as_deref()
                .and_then(|p| std::path::Path::new(p).file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "No .glyph file chosen".to_string());

            let file_picker = row![
                container(
                    text(file_label)
                        .size(12)
                        .color(if st.file_path.is_some() { TEXT_PRIMARY } else { TEXT_MUTED })
                )
                .width(Length::Fill)
                .padding([9, 12])
                .style(dark_card),
                button(text("Choose .glyph File...").size(12).color(TEXT_LIGHT))
                    .padding([9, 14])
                    .style(secondary_button)
                    .on_press(ModalMsg::ImportBrowseFile),
            ]
            .spacing(8)
            .align_y(alignment::Vertical::Center);

            let pw_row = row![
                text_input("Enter backup master password...", &st.password)
                    .secure(!st.show_password)
                    .size(13)
                    .padding([9, 12])
                    .style(dark_input)
                    .on_input(ModalMsg::ImportPasswordChanged)
                    .on_submit(ModalMsg::ImportDecryptSubmit),
                button(if st.show_password {
                    icons::eye_off(16.0, TEXT_SECONDARY)
                } else {
                    icons::eye(16.0, TEXT_SECONDARY)
                })
                .padding([8, 10])
                .style(secondary_button)
                .on_press(ModalMsg::ImportToggleShowPassword),
            ]
            .spacing(8);

            content = content
                .push(text("Backup File").size(12).font(FONT_MONO_BOLD).color(TEXT_LIGHT))
                .push(file_picker)
                .push(
                    text("Master Decryption Password")
                        .size(12)
                        .font(FONT_MONO_BOLD)
                        .color(TEXT_LIGHT),
                )
                .push(pw_row);

            if let Some(err) = &st.error {
                content = content.push(error_banner(err));
            }

            let can_decrypt =
                !st.busy && st.encrypted_content.is_some() && !st.password.is_empty();
            let next_btn = button(
                text(if st.busy {
                    "Decrypting..."
                } else {
                    "Decrypt & Preview"
                })
                .size(13)
                .font(FONT_MONO_BOLD)
                .color(Color::WHITE),
            )
            .padding([9, 16])
            .style(brand_button);
            let next_btn = if can_decrypt {
                next_btn.on_press(ModalMsg::ImportDecryptSubmit)
            } else {
                next_btn
            };

            content = content.push(
                row![
                    space().width(Length::Fill),
                    button(text("Cancel").size(13).color(TEXT_LIGHT))
                        .padding([9, 16])
                        .style(secondary_button)
                        .on_press(ModalMsg::Close),
                    next_btn,
                ]
                .spacing(10),
            );
        }
        ImportStep::SelectServers(list) => {
            let sel_bar = row![
                text(format!(
                    "Select Servers to Import ({} / {})",
                    st.selected_indices.len(),
                    list.len()
                ))
                .size(12)
                .font(FONT_MONO_BOLD)
                .color(TEXT_LIGHT),
                space().width(Length::Fill),
                button(text("Select All").size(11).color(BRAND_400))
                    .padding([3, 8])
                    .style(ghost_icon_button)
                    .on_press(ModalMsg::ImportSelectAll),
                button(text("Deselect All").size(11).color(TEXT_SECONDARY))
                    .padding([3, 8])
                    .style(ghost_icon_button)
                    .on_press(ModalMsg::ImportDeselectAll),
            ]
            .spacing(8)
            .align_y(alignment::Vertical::Center);

            let mut list_col = column![].spacing(6);
            for (idx, srv) in list.iter().enumerate() {
                let checked = st.selected_indices.contains(&idx);
                let title = if srv.name.is_empty() { &srv.host } else { &srv.name };
                let label = format!("{}  ({}@{}:{})", title, srv.username, srv.host, srv.port);
                list_col = list_col.push(
                    container(
                        checkbox(checked)
                            .label(label)
                            .size(16.0)
                            .text_size(12.0)
                            .on_toggle(move |v| ModalMsg::ImportToggleServer(idx, v)),
                    )
                    .width(Length::Fill)
                    .padding([8, 10])
                    .style(move |_| container::Style {
                        background: Some(Background::Color(if checked {
                            rgba(BRAND_500, 0.1)
                        } else {
                            DARK_800
                        })),
                        border: Border {
                            color: if checked {
                                rgba(BRAND_500, 0.35)
                            } else {
                                DARK_700
                            },
                            width: 1.0,
                            radius: 8.0.into(),
                        },
                        ..Default::default()
                    }),
                );
            }

            let list_scroll = container(
                scrollable(list_col)
                    .height(Length::Fixed(220.0))
                    .style(dark_scrollable),
            )
            .padding(8)
            .style(|_| container::Style {
                background: Some(Background::Color(DARK_900)),
                border: Border {
                    color: DARK_700,
                    width: 1.0,
                    radius: 10.0.into(),
                },
                ..Default::default()
            });

            content = content.push(sel_bar).push(list_scroll);
            if let Some(err) = &st.error {
                content = content.push(error_banner(err));
            }

            let count = st.selected_indices.len();
            let import_btn = button(
                text(format!("Import ({count})"))
                    .size(13)
                    .font(FONT_MONO_BOLD)
                    .color(Color::WHITE),
            )
            .padding([9, 16])
            .style(brand_button);
            let import_btn = if count > 0 && !st.busy {
                import_btn.on_press(ModalMsg::ImportConfirmSubmit)
            } else {
                import_btn
            };

            content = content.push(
                row![
                    button(text("← Back").size(13).color(TEXT_LIGHT))
                        .padding([9, 14])
                        .style(secondary_button)
                        .on_press(ModalMsg::ImportBackToUnlock),
                    space().width(Length::Fill),
                    button(text("Cancel").size(13).color(TEXT_LIGHT))
                        .padding([9, 16])
                        .style(secondary_button)
                        .on_press(ModalMsg::Close),
                    import_btn,
                ]
                .spacing(10),
            );
        }
    }

    container(content)
        .width(Length::Fixed(520.0))
        .padding(24)
        .style(modal_card)
        .into()
}

fn view_ssh_keys<'a>(
    st: &'a SshKeysModalState,
    servers: &'a [ServerSummary],
    ssh_keys: &'a [SshKeySummary],
) -> Element<'a, ModalMsg> {
    let header = row![
        container(icons::key(18.0, BRAND_400))
            .padding(9)
            .style(|_| container::Style {
                background: Some(Background::Color(rgba(BRAND_500, 0.14))),
                border: Border {
                    color: rgba(BRAND_500, 0.3),
                    width: 1.0,
                    radius: 10.0.into(),
                },
                ..Default::default()
            }),
        column![
            text("SSH Key Vault")
                .size(18)
                .font(FONT_MONO_BOLD)
                .color(TEXT_PRIMARY),
            text("Generate, import, copy public keys, and 1-click deploy to servers")
                .size(11)
                .color(TEXT_SECONDARY),
        ]
        .spacing(2),
        space().width(Length::Fill),
        button(
            row![
                icons::plus(13.0, Color::WHITE),
                text("Generate Key").size(12).font(FONT_MONO_BOLD).color(Color::WHITE),
            ]
            .spacing(5)
            .align_y(alignment::Vertical::Center)
        )
        .padding([7, 12])
        .style(brand_button)
        .on_press(ModalMsg::SshKeyToggleGenerate),
        button(
            row![
                icons::upload(13.0, TEXT_LIGHT),
                text("Import Key").size(12).color(TEXT_LIGHT),
            ]
            .spacing(5)
            .align_y(alignment::Vertical::Center)
        )
        .padding([7, 12])
        .style(secondary_button)
        .on_press(ModalMsg::SshKeyToggleImport),
        button(icons::x_close(16.0, TEXT_SECONDARY))
            .padding(6)
            .style(ghost_icon_button)
            .on_press(ModalMsg::Close),
    ]
    .spacing(10)
    .align_y(alignment::Vertical::Center);

    let mut content = column![header].spacing(14);

    if st.show_generate {
        const ALGOS: [SshKeyAlgorithm; 3] = [
            SshKeyAlgorithm::Ed25519,
            SshKeyAlgorithm::Rsa4096,
            SshKeyAlgorithm::EcdsaP256,
        ];
        let gen_form = container(
            column![
                text("Generate New SSH Keypair")
                    .size(13)
                    .font(FONT_MONO_BOLD)
                    .color(BRAND_300),
                row![
                    text_input("Key label (e.g. prod-ed25519)", &st.gen_name)
                        .size(12)
                        .padding([8, 10])
                        .style(dark_input)
                        .on_input(ModalMsg::SshKeyGenName),
                    pick_list(ALGOS.as_slice(), Some(st.gen_algo), ModalMsg::SshKeyGenAlgo)
                        .padding([7, 10])
                        .text_size(12)
                        .style(dark_pick_list),
                ]
                .spacing(8),
                row![
                    text_input(
                        "Optional passphrase (leave empty for passwordless)",
                        &st.gen_passphrase
                    )
                    .secure(!st.show_gen_pass)
                    .size(12)
                    .padding([8, 10])
                    .style(dark_input)
                    .on_input(ModalMsg::SshKeyGenPass),
                    button(if st.show_gen_pass {
                        icons::eye_off(15.0, TEXT_SECONDARY)
                    } else {
                        icons::eye(15.0, TEXT_SECONDARY)
                    })
                    .padding([7, 9])
                    .style(secondary_button)
                    .on_press(ModalMsg::SshKeyGenToggleShowPass),
                    button(
                        text("Generate & Save")
                            .size(12)
                            .font(FONT_MONO_BOLD)
                            .color(Color::WHITE)
                    )
                    .padding([8, 14])
                    .style(brand_button)
                    .on_press(ModalMsg::SshKeyGenSubmit),
                ]
                .spacing(8)
                .align_y(alignment::Vertical::Center),
            ]
            .spacing(10),
        )
        .padding(14)
        .style(|_| container::Style {
            background: Some(Background::Color(rgba(BRAND_500, 0.06))),
            border: Border {
                color: rgba(BRAND_500, 0.35),
                width: 1.0,
                radius: 10.0.into(),
            },
            ..Default::default()
        });
        content = content.push(gen_form);
    }

    if st.show_import {
        let imp_file_label = st
            .imp_path
            .as_deref()
            .and_then(|p| std::path::Path::new(p).file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Select OpenSSH private key (id_ed25519 / id_rsa)...".into());
        let imp_form = container(
            column![
                text("Import Existing OpenSSH Private Key")
                    .size(13)
                    .font(FONT_MONO_BOLD)
                    .color(BRAND_300),
                row![
                    text_input("Key label (e.g. legacy-ops)", &st.imp_name)
                        .size(12)
                        .padding([8, 10])
                        .style(dark_input)
                        .on_input(ModalMsg::SshKeyImpName),
                    button(text(imp_file_label).size(12).color(TEXT_LIGHT))
                        .padding([8, 12])
                        .style(secondary_button)
                        .on_press(ModalMsg::SshKeyImpBrowseFile),
                ]
                .spacing(8)
                .align_y(alignment::Vertical::Center),
                row![
                    text_input("Key passphrase (if encrypted)", &st.imp_passphrase)
                        .secure(!st.show_imp_pass)
                        .size(12)
                        .padding([8, 10])
                        .style(dark_input)
                        .on_input(ModalMsg::SshKeyImpPass),
                    button(if st.show_imp_pass {
                        icons::eye_off(15.0, TEXT_SECONDARY)
                    } else {
                        icons::eye(15.0, TEXT_SECONDARY)
                    })
                    .padding([7, 9])
                    .style(secondary_button)
                    .on_press(ModalMsg::SshKeyImpToggleShowPass),
                    button(
                        text("Import Key")
                            .size(12)
                            .font(FONT_MONO_BOLD)
                            .color(Color::WHITE)
                    )
                    .padding([8, 14])
                    .style(brand_button)
                    .on_press(ModalMsg::SshKeyImpSubmit),
                ]
                .spacing(8)
                .align_y(alignment::Vertical::Center),
            ]
            .spacing(10),
        )
        .padding(14)
        .style(dark_card);
        content = content.push(imp_form);
    }

    if let Some(err) = &st.error {
        content = content.push(error_banner(err));
    }
    if let Some(ok) = &st.status {
        content = content.push(success_banner(ok));
    }

    let server_options: Vec<ServerPickOption> = servers
        .iter()
        .map(|s| ServerPickOption {
            id: s.id.clone(),
            label: format!(
                "{} ({}@{}:{})",
                if s.name.is_empty() { &s.host } else { &s.name },
                s.username,
                s.host,
                s.port
            ),
        })
        .collect();

    let mut keys_col = column![].spacing(10);
    if ssh_keys.is_empty() {
        keys_col = keys_col.push(
            container(
                column![
                    icons::key(28.0, TEXT_DIM),
                    text("No SSH keys stored in your encrypted vault yet.")
                        .size(13)
                        .color(TEXT_SECONDARY),
                    text("Click \"+ Generate Key\" to create an Ed25519 / RSA-4096 / ECDSA keypair.")
                        .size(11)
                        .color(TEXT_MUTED),
                ]
                .spacing(6)
                .align_x(alignment::Horizontal::Center),
            )
            .width(Length::Fill)
            .padding(28)
            .align_x(alignment::Horizontal::Center)
            .style(dark_card),
        );
    } else {
        for k in ssh_keys {
            let copied = st.copied_key_id.as_deref() == Some(k.id.as_str());
            let deploying_this = st.deploy_key_id.as_deref() == Some(k.id.as_str());
            let kid_copy = k.id.clone();
            let kid_deploy = k.id.clone();
            let kid_del = k.id.clone();

            let mut card_col = column![
                row![
                    text(&k.name).size(14).font(FONT_MONO_BOLD).color(TEXT_PRIMARY),
                    container(
                        text(&k.algorithm)
                            .size(10)
                            .font(FONT_MONO_BOLD)
                            .color(BRAND_300)
                    )
                    .padding([2, 8])
                    .style(pill_badge(rgba(BRAND_500, 0.14), rgba(BRAND_500, 0.35), BRAND_300)),
                    if k.has_passphrase {
                        Element::from(
                            container(text("LOCKED").size(10).color(YELLOW_400))
                                .padding([2, 8])
                                .style(pill_badge(
                                    rgba(YELLOW_500, 0.14),
                                    rgba(YELLOW_500, 0.35),
                                    YELLOW_400,
                                )),
                        )
                    } else {
                        space().width(0).into()
                    },
                    space().width(Length::Fill),
                    button(
                        row![
                            if copied {
                                icons::check(13.0, GREEN_400)
                            } else {
                                icons::copy(13.0, TEXT_LIGHT)
                            },
                            text(if copied { "Copied PubKey" } else { "Copy Public Key" })
                                .size(11)
                                .color(if copied { GREEN_400 } else { TEXT_LIGHT }),
                        ]
                        .spacing(5)
                        .align_y(alignment::Vertical::Center),
                    )
                    .padding([5, 10])
                    .style(secondary_button)
                    .on_press(ModalMsg::SshKeyCopyPublic(kid_copy)),
                    button(
                        row![
                            icons::server(13.0, BRAND_300),
                            text("Deploy to Server").size(11).color(BRAND_300),
                        ]
                        .spacing(5)
                        .align_y(alignment::Vertical::Center),
                    )
                    .padding([5, 10])
                    .style(secondary_button)
                    .on_press(ModalMsg::SshKeyToggleDeploy(kid_deploy)),
                    button(icons::trash(14.0, RED_400))
                        .padding(6)
                        .style(danger_ghost_button)
                        .on_press(ModalMsg::SshKeyDelete(kid_del)),
                ]
                .spacing(8)
                .align_y(alignment::Vertical::Center),
                text(&k.fingerprint_sha256)
                    .size(11)
                    .font(FONT_MONO)
                    .color(TEXT_MUTED),
            ]
            .spacing(6);

            if deploying_this {
                let selected_srv = st
                    .deploy_server_id
                    .as_deref()
                    .and_then(|sid| server_options.iter().find(|o| o.id == sid))
                    .cloned();
                let deploy_btn = button(
                    text(if st.busy {
                        "Deploying..."
                    } else {
                        "Install Public Key"
                    })
                    .size(12)
                    .font(FONT_MONO_BOLD)
                    .color(Color::WHITE),
                )
                .padding([7, 12])
                .style(brand_button);
                let deploy_btn = if !st.busy && selected_srv.is_some() {
                    deploy_btn.on_press(ModalMsg::SshKeyDeploySubmit)
                } else {
                    deploy_btn
                };

                card_col = card_col.push(
                    container(
                        row![
                            pick_list(
                                server_options.clone(),
                                selected_srv,
                                ModalMsg::SshKeySelectDeployServer
                            )
                            .placeholder("Choose saved server to authorize...")
                            .width(Length::Fill)
                            .padding([6, 10])
                            .text_size(12)
                            .style(dark_pick_list),
                            deploy_btn,
                        ]
                        .spacing(8)
                        .align_y(alignment::Vertical::Center),
                    )
                    .padding(10)
                    .style(|_| container::Style {
                        background: Some(Background::Color(DARK_800)),
                        border: Border {
                            color: rgba(BRAND_500, 0.3),
                            width: 1.0,
                            radius: 8.0.into(),
                        },
                        ..Default::default()
                    }),
                );
            }

            keys_col = keys_col.push(
                container(card_col)
                    .width(Length::Fill)
                    .padding(14)
                    .style(dark_card),
            );
        }
    }

    content = content.push(
        scrollable(keys_col)
            .height(Length::Fixed(310.0))
            .style(dark_scrollable),
    );

    container(content)
        .width(Length::Fixed(720.0))
        .padding(24)
        .style(modal_card)
        .into()
}

fn view_connecting<'a>(st: &'a ConnectingModalState) -> Element<'a, ModalMsg> {
    let badge = if st.error.is_some() {
        container(
            text("FAILED")
                .size(11)
                .font(FONT_MONO_BOLD)
                .color(RED_400),
        )
        .padding([3, 10])
        .style(pill_badge(rgba(RED_500, 0.16), rgba(RED_500, 0.35), RED_400))
    } else if st.host_key_prompt.is_some() {
        container(
            text("VERIFY HOST")
                .size(11)
                .font(FONT_MONO_BOLD)
                .color(YELLOW_400),
        )
        .padding([3, 10])
        .style(pill_badge(
            rgba(YELLOW_500, 0.16),
            rgba(YELLOW_500, 0.35),
            YELLOW_400,
        ))
    } else {
        container(
            text(format!("{} ms", st.elapsed_ms))
                .size(11)
                .font(FONT_MONO_BOLD)
                .color(BRAND_300),
        )
        .padding([3, 10])
        .style(pill_badge(rgba(BRAND_500, 0.16), rgba(BRAND_500, 0.35), BRAND_300))
    };

    let header = row![
        os_logo(st.os.as_deref(), &st.server_name, 42.0),
        column![
            row![
                text(if st.server_name.is_empty() {
                    &st.host
                } else {
                    &st.server_name
                })
                .size(17)
                .font(FONT_MONO_BOLD)
                .color(TEXT_PRIMARY),
                badge,
            ]
            .spacing(10)
            .align_y(alignment::Vertical::Center),
            text(format!("{}@{}:{}", st.username, st.host, st.port))
                .size(12)
                .font(FONT_MONO)
                .color(TEXT_SECONDARY),
        ]
        .spacing(4),
        space().width(Length::Fill),
        button(icons::x_close(16.0, TEXT_SECONDARY))
            .padding(6)
            .style(ghost_icon_button)
            .on_press(ModalMsg::Close),
    ]
    .spacing(14)
    .align_y(alignment::Vertical::Center);

    let mut log_col = column![].spacing(4);
    for (idx, line) in st.logs.iter().enumerate() {
        let is_last = idx + 1 == st.logs.len();
        let line_txt = if is_last && st.error.is_none() && st.host_key_prompt.is_none() && st.blink_on {
            format!("❯ {line} █")
        } else {
            format!("❯ {line}")
        };
        log_col = log_col.push(
            text(line_txt)
                .size(12)
                .font(FONT_MONO)
                .color(if is_last && st.error.is_some() {
                    RED_400
                } else if is_last {
                    TEXT_PRIMARY
                } else {
                    TEXT_SECONDARY
                }),
        );
    }

    let log_box = container(
        scrollable(log_col)
            .height(Length::Fixed(160.0))
            .style(dark_scrollable),
    )
    .width(Length::Fill)
    .padding(14)
    .style(|_| container::Style {
        background: Some(Background::Color(DARK_900)),
        border: Border {
            color: DARK_700,
            width: 1.0,
            radius: 10.0.into(),
        },
        ..Default::default()
    });

    let mut content = column![header, log_box].spacing(16);

    if let Some(prompt) = &st.host_key_prompt {
        let is_mismatch = prompt.mismatch_expected.is_some();
        let accent = if is_mismatch { RED_400 } else { ORANGE_400 };
        let pid = prompt.prompt_id;

        let mut prompt_col = column![
            row![
                if is_mismatch {
                    icons::alert_triangle(18.0, RED_400)
                } else {
                    icons::shield_check(18.0, ORANGE_400)
                },
                text(if is_mismatch {
                    "WARNING: Remote Host Key Changed! (Possible MITM)"
                } else {
                    "Trust New SSH Host Key (TOFU Verification)"
                })
                .size(13)
                .font(FONT_MONO_BOLD)
                .color(accent),
            ]
            .spacing(8)
            .align_y(alignment::Vertical::Center),
            text(format!(
                "Host: {}:{}   ·   Algorithm: {}",
                prompt.host, prompt.port, prompt.algorithm
            ))
            .size(12)
            .color(TEXT_LIGHT),
            text(format!("Fingerprint: {}", prompt.fingerprint_sha256))
                .size(11)
                .font(FONT_MONO)
                .color(TEXT_PRIMARY),
        ]
        .spacing(6);

        if let Some(expected) = &prompt.mismatch_expected {
            prompt_col = prompt_col.push(
                text(format!("Previously trusted: {expected}"))
                    .size(11)
                    .font(FONT_MONO)
                    .color(RED_400),
            );
        }

        let trust_btn = button(
            text(if is_mismatch {
                "Override & Trust New Key"
            } else {
                "Trust & Connect"
            })
            .size(12)
            .font(FONT_MONO_BOLD)
            .color(Color::WHITE),
        )
        .padding([8, 14])
        .on_press(ModalMsg::RespondHostKey(pid, true));
        let trust_btn = if is_mismatch {
            trust_btn.style(danger_button)
        } else {
            trust_btn.style(brand_button)
        };

        prompt_col = prompt_col.push(
            row![
                space().width(Length::Fill),
                button(text("Reject").size(12).color(TEXT_LIGHT))
                    .padding([8, 14])
                    .style(secondary_button)
                    .on_press(ModalMsg::RespondHostKey(pid, false)),
                trust_btn,
            ]
            .spacing(8),
        );

        let prompt_box = container(prompt_col)
            .width(Length::Fill)
            .padding(14)
            .style(move |_| container::Style {
                background: Some(Background::Color(rgba(accent, 0.08))),
                border: Border {
                    color: rgba(accent, 0.4),
                    width: 1.0,
                    radius: 10.0.into(),
                },
                ..Default::default()
            });
        content = content.push(prompt_box);
    }

    if let Some(err) = &st.error {
        content = content
            .push(error_banner(err))
            .push(
                row![
                    space().width(Length::Fill),
                    button(text("Close").size(13).color(TEXT_LIGHT))
                        .padding([8, 16])
                        .style(secondary_button)
                        .on_press(ModalMsg::Close),
                    button(
                        text("Retry Connection")
                            .size(13)
                            .font(FONT_MONO_BOLD)
                            .color(Color::WHITE)
                    )
                    .padding([8, 16])
                    .style(brand_button)
                    .on_press(ModalMsg::ConnectingRetry),
                ]
                .spacing(10),
            );
    }

    container(content)
        .width(Length::Fixed(560.0))
        .padding(24)
        .style(modal_card)
        .into()
}

fn view_settings<'a>(
    st: &'a SettingsModalState,
    known_hosts: &'a [KnownHostEntry],
) -> Element<'a, ModalMsg> {
    let header = row![
        container(icons::settings(18.0, BRAND_400))
            .padding(9)
            .style(|_| container::Style {
                background: Some(Background::Color(rgba(BRAND_500, 0.14))),
                border: Border {
                    color: rgba(BRAND_500, 0.3),
                    width: 1.0,
                    radius: 10.0.into(),
                },
                ..Default::default()
            }),
        column![
            text("Settings")
                .size(18)
                .font(FONT_MONO_BOLD)
                .color(TEXT_PRIMARY),
            text(format!("Glyph v{APP_VERSION} · Pure Rust Edition"))
                .size(11)
                .color(TEXT_SECONDARY),
        ]
        .spacing(2),
        space().width(Length::Fill),
        button(icons::x_close(16.0, TEXT_SECONDARY))
            .padding(6)
            .style(ghost_icon_button)
            .on_press(ModalMsg::Close),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center);

    let nav_btn = |tab: SettingsTab, label: &'static str, icon_el: Element<'a, ModalMsg>| {
        let active = st.active_tab == tab;
        button(
            row![
                icon_el,
                text(label)
                    .size(12)
                    .font(if active { FONT_MONO_BOLD } else { FONT_MONO })
                    .color(if active { BRAND_300 } else { TEXT_LIGHT }),
            ]
            .spacing(10)
            .align_y(alignment::Vertical::Center),
        )
        .width(Length::Fill)
        .padding([10, 12])
        .style(move |_, status| {
            let bg = if active {
                rgba(BRAND_500, 0.16)
            } else if matches!(status, button::Status::Hovered) {
                DARK_700
            } else {
                Color::TRANSPARENT
            };
            button::Style {
                background: Some(Background::Color(bg)),
                border: Border {
                    color: if active {
                        rgba(BRAND_500, 0.4)
                    } else {
                        Color::TRANSPARENT
                    },
                    width: if active { 1.0 } else { 0.0 },
                    radius: 8.0.into(),
                },
                ..Default::default()
            }
        })
        .on_press(ModalMsg::SelectSettingsTab(tab))
    };

    let sidebar = container(
        column![
            nav_btn(
                SettingsTab::AiAgents,
                "AI Agents (MCP)",
                icons::sparkles(15.0, BRAND_400).into(),
            ),
            nav_btn(
                SettingsTab::Security,
                "Security & Hosts",
                icons::shield_check(15.0, GREEN_400).into(),
            ),
            nav_btn(
                SettingsTab::Mobile,
                "Glyph Mobile",
                icons::smartphone(15.0, PURPLE_400).into(),
            ),
            nav_btn(
                SettingsTab::Updates,
                "Updates",
                icons::refresh_cw(15.0, ORANGE_400).into(),
            ),
        ]
        .spacing(6),
    )
    .width(Length::Fixed(185.0))
    .height(Length::Fill)
    .padding(12)
    .style(|_| container::Style {
        background: Some(Background::Color(DARK_800)),
        border: Border {
            color: DARK_700,
            width: 1.0,
            radius: 12.0.into(),
        },
        ..Default::default()
    });

    let right_pane: Element<'a, ModalMsg> = match st.active_tab {
        SettingsTab::AiAgents => view_settings_mcp(st),
        SettingsTab::Security => view_settings_security(st, known_hosts),
        SettingsTab::Mobile => view_settings_mobile(),
        SettingsTab::Updates => view_settings_updates(st),
    };

    let split = row![
        sidebar,
        container(scrollable(right_pane).height(Length::Fill).style(dark_scrollable))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding([0, 4]),
    ]
    .spacing(16)
    .height(Length::Fixed(470.0));

    let content = column![
        header,
        split,
        row![
            space().width(Length::Fill),
            button(text("Done").size(13).font(FONT_MONO_BOLD).color(Color::WHITE))
                .padding([8, 20])
                .style(brand_button)
                .on_press(ModalMsg::Close),
        ],
    ]
    .spacing(14);

    container(content)
        .width(Length::Fixed(780.0))
        .padding(24)
        .style(modal_card)
        .into()
}

fn view_settings_mcp<'a>(st: &'a SettingsModalState) -> Element<'a, ModalMsg> {
    let mut col = column![
        row![
            column![
                text("Model Context Protocol (MCP)")
                    .size(15)
                    .font(FONT_MONO_BOLD)
                    .color(TEXT_PRIMARY),
                text("Allow Antigravity, Claude, Cursor, or VS Code to operate your connected servers securely.")
                    .size(11)
                    .color(TEXT_SECONDARY),
            ]
            .spacing(3),
            space().width(Length::Fill),
            button(
                row![
                    icons::refresh_cw(13.0, TEXT_LIGHT),
                    text("Rescan").size(11).color(TEXT_LIGHT),
                ]
                .spacing(5)
                .align_y(alignment::Vertical::Center),
            )
            .padding([6, 10])
            .style(secondary_button)
            .on_press(ModalMsg::RefreshMcpClients),
        ]
        .align_y(alignment::Vertical::Center),
        container(
            column![
                row![
                    container(text("Local API Active").size(10).font(FONT_MONO_BOLD).color(GREEN_400))
                        .padding([2, 8])
                        .style(pill_badge(rgba(GREEN_500, 0.14), rgba(GREEN_500, 0.35), GREEN_400)),
                    text(format!("127.0.0.1:{}", st.api_port))
                        .size(12)
                        .font(FONT_MONO_BOLD)
                        .color(TEXT_LIGHT),
                ]
                .spacing(8)
                .align_y(alignment::Vertical::Center),
                text(format!("MCP Binary: {}", st.mcp_bin_path))
                    .size(10)
                    .font(FONT_MONO)
                    .color(TEXT_MUTED),
                text(format!("Bearer Token: {}", st.mcp_token_path))
                    .size(10)
                    .font(FONT_MONO)
                    .color(TEXT_MUTED),
            ]
            .spacing(4),
        )
        .width(Length::Fill)
        .padding(12)
        .style(dark_card),
    ]
    .spacing(10);

    if let Some(msg) = &st.mcp_status {
        col = col.push(success_banner(msg));
    }
    if let Some(err) = &st.mcp_error {
        col = col.push(error_banner(err));
    }

    for c in &st.clients {
        let (badge_lbl, badge_col, badge_bg) = match c.status {
            McpClientStatus::Configured => ("Configured", GREEN_400, rgba(GREEN_500, 0.14)),
            McpClientStatus::NotConfigured => ("Not set up", YELLOW_400, rgba(YELLOW_500, 0.14)),
            McpClientStatus::NotInstalled => ("Not installed", TEXT_MUTED, rgba(DARK_600, 0.35)),
        };

        let expanded = st.expanded_snippet == Some(c.id);
        let copied = st.copied_snippet == Some(c.id);
        let btn_lbl = if c.status == McpClientStatus::Configured {
            "Re-apply"
        } else {
            "Auto-Install"
        };

        let mut card_col = column![
            row![
                container(
                    text(c.id.badge_tag())
                        .size(10)
                        .font(FONT_MONO_BOLD)
                        .color(BRAND_300),
                )
                .padding([4, 7])
                .style(|_| container::Style {
                    background: Some(Background::Color(rgba(BRAND_500, 0.16))),
                    border: Border {
                        color: rgba(BRAND_500, 0.3),
                        width: 1.0,
                        radius: 6.0.into(),
                    },
                    ..Default::default()
                }),
                column![
                    text(c.name)
                        .size(13)
                        .font(FONT_MONO_BOLD)
                        .color(TEXT_PRIMARY),
                    text(c.config_path.display().to_string())
                        .size(10)
                        .font(FONT_MONO)
                        .color(TEXT_MUTED),
                ]
                .spacing(2),
                space().width(Length::Fill),
                container(
                    text(badge_lbl)
                        .size(10)
                        .font(FONT_MONO_BOLD)
                        .color(badge_col),
                )
                .padding([3, 8])
                .style(pill_badge(badge_bg, rgba(badge_col, 0.35), badge_col)),
                button(text("JSON").size(11).color(TEXT_LIGHT))
                    .padding([5, 9])
                    .style(secondary_button)
                    .on_press(ModalMsg::ToggleMcpSnippet(c.id)),
                button(
                    text(btn_lbl)
                        .size(11)
                        .font(FONT_MONO_BOLD)
                        .color(Color::WHITE),
                )
                .padding([5, 11])
                .style(brand_button)
                .on_press(ModalMsg::AutoInstallMcp(c.id)),
            ]
            .spacing(8)
            .align_y(alignment::Vertical::Center),
        ]
        .spacing(8);

        if expanded {
            let snippet = build_config_snippet(c.format, None, Path::new(&st.mcp_bin_path));
            card_col = card_col.push(
                container(
                    column![
                        row![
                            text("Config Snippet").size(10).color(TEXT_MUTED),
                            space().width(Length::Fill),
                            button(
                                row![
                                    if copied {
                                        icons::check(11.0, GREEN_400)
                                    } else {
                                        icons::copy(11.0, TEXT_SECONDARY)
                                    },
                                    text(if copied { "Copied" } else { "Copy" })
                                        .size(10)
                                        .color(if copied { GREEN_400 } else { TEXT_LIGHT }),
                                ]
                                .spacing(4)
                                .align_y(alignment::Vertical::Center),
                            )
                            .padding([3, 8])
                            .style(secondary_button)
                            .on_press(ModalMsg::CopyMcpSnippet(c.id)),
                        ]
                        .align_y(alignment::Vertical::Center),
                        text(snippet).size(10).font(FONT_MONO).color(TEXT_LIGHT),
                    ]
                    .spacing(6),
                )
                .width(Length::Fill)
                .padding(10)
                .style(|_| container::Style {
                    background: Some(Background::Color(DARK_900)),
                    border: Border {
                        color: DARK_700,
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                }),
            );
        }

        col = col.push(
            container(card_col)
                .width(Length::Fill)
                .padding(12)
                .style(|_| container::Style {
                    background: Some(Background::Color(DARK_800)),
                    border: Border {
                        color: DARK_700,
                        width: 1.0,
                        radius: 10.0.into(),
                    },
                    ..Default::default()
                }),
        );
    }

    // Collapsible Custom / Manual Setup
    let custom_toggle = button(
        row![
            icons::sliders(13.0, BRAND_300),
            text("Custom / Manual Config Writer")
                .size(12)
                .font(FONT_MONO_BOLD)
                .color(BRAND_300),
            space().width(Length::Fill),
            icons::chevron_down(14.0, TEXT_MUTED),
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center),
    )
    .width(Length::Fill)
    .padding([10, 12])
    .style(secondary_button)
    .on_press(ModalMsg::ToggleCustomMcp);

    col = col.push(custom_toggle);

    if st.show_custom {
        let formats = vec![
            McpConfigFormat::McpServers,
            McpConfigFormat::VsCode,
            McpConfigFormat::Custom,
        ];
        let preview = build_config_snippet(
            st.custom_format,
            Some(st.custom_key.as_str()),
            Path::new(&st.mcp_bin_path),
        );
        let mut custom_col = column![
            row![
                text_input("Path to config.json / settings.json", &st.custom_path)
                    .on_input(ModalMsg::CustomMcpPathChanged)
                    .padding([8, 10])
                    .size(12)
                    .font(FONT_MONO)
                    .width(Length::Fill)
                    .style(dark_input),
                button(text("Browse").size(12).color(TEXT_LIGHT))
                    .padding([8, 12])
                    .style(secondary_button)
                    .on_press(ModalMsg::BrowseCustomMcpPath),
            ]
            .spacing(8),
            pick_list(formats, Some(st.custom_format), ModalMsg::CustomMcpFormatChanged)
                .padding([8, 12])
                .text_size(12)
                .width(Length::Fill)
                .style(dark_pick_list),
        ]
        .spacing(8);

        if st.custom_format == McpConfigFormat::Custom {
            custom_col = custom_col.push(
                text_input("Dot-separated key path (e.g. mcpServers or tools.mcp)", &st.custom_key)
                    .on_input(ModalMsg::CustomMcpKeyChanged)
                    .padding([8, 10])
                    .size(12)
                    .font(FONT_MONO)
                    .width(Length::Fill)
                    .style(dark_input),
            );
        }

        custom_col = custom_col
            .push(
                container(text(preview).size(10).font(FONT_MONO).color(TEXT_LIGHT))
                    .width(Length::Fill)
                    .padding(10)
                    .style(dark_card),
            )
            .push(
                row![
                    space().width(Length::Fill),
                    button(
                        text("Write to Config")
                            .size(12)
                            .font(FONT_MONO_BOLD)
                            .color(Color::WHITE),
                    )
                    .padding([8, 14])
                    .style(brand_button)
                    .on_press(ModalMsg::WriteCustomMcp),
                ],
            );

        col = col.push(
            container(custom_col)
                .width(Length::Fill)
                .padding(12)
                .style(dark_card),
        );
    }

    col.into()
}

fn view_settings_security<'a>(
    st: &'a SettingsModalState,
    known_hosts: &'a [KnownHostEntry],
) -> Element<'a, ModalMsg> {
    let vault_card = container(
        column![
            row![
                icons::shield_check(
                    16.0,
                    if st.encryption_available { GREEN_400 } else { YELLOW_400 }
                ),
                text("Encrypted Vault Status")
                    .size(13)
                    .font(FONT_MONO_BOLD)
                    .color(TEXT_PRIMARY),
                space().width(Length::Fill),
                text(if st.encryption_available {
                    "OS Keyring Active"
                } else {
                    "Machine-Wrapped Key File"
                })
                .size(11)
                .color(if st.encryption_available { GREEN_400 } else { YELLOW_400 }),
            ]
            .spacing(8)
            .align_y(alignment::Vertical::Center),
            text(format!("Vault location: {}", st.data_dir))
                .size(11)
                .font(FONT_MONO)
                .color(TEXT_MUTED),
        ]
        .spacing(6),
    )
    .width(Length::Fill)
    .padding(12)
    .style(dark_card);

    let mut kh_col = column![].spacing(6);
    if known_hosts.is_empty() {
        kh_col = kh_col.push(
            text("No SSH host keys trusted yet (TOFU populates on first connect).")
                .size(11)
                .color(TEXT_MUTED),
        );
    } else {
        for h in known_hosts {
            let hp = h.host_port.clone();
            kh_col = kh_col.push(
                container(
                    row![
                        column![
                            text(format!("{}  ({})", h.host_port, h.algorithm))
                                .size(12)
                                .font(FONT_MONO_BOLD)
                                .color(TEXT_LIGHT),
                            text(&h.fingerprint_sha256)
                                .size(10)
                                .font(FONT_MONO)
                                .color(TEXT_MUTED),
                        ]
                        .spacing(2),
                        space().width(Length::Fill),
                        button(icons::trash(13.0, RED_400))
                            .padding(6)
                            .style(danger_ghost_button)
                            .on_press(ModalMsg::RemoveKnownHost(hp)),
                    ]
                    .align_y(alignment::Vertical::Center),
                )
                .width(Length::Fill)
                .padding([8, 10])
                .style(dark_card),
            );
        }
    }

    column![
        vault_card,
        text("Trusted SSH Host Keys (TOFU Known Hosts)")
            .size(12)
            .font(FONT_MONO_BOLD)
            .color(TEXT_LIGHT),
        container(kh_col)
            .width(Length::Fill)
            .padding(10)
            .style(dark_card),
    ]
    .spacing(12)
    .into()
}

fn view_settings_mobile<'a>() -> Element<'a, ModalMsg> {
    container(
        column![
            icons::smartphone(38.0, BRAND_400),
            text("Glyph Mobile Companion (Android APK)")
                .size(16)
                .font(FONT_MONO_BOLD)
                .color(TEXT_PRIMARY),
            text("Carry your SSH servers, live 5-metric telemetry, PTY terminal, SFTP editor, and Docker manager on Android with full AES-256-GCM .glyph backup interoperability.")
                .size(12)
                .color(TEXT_SECONDARY),
            button(
                row![
                    icons::qr_code(15.0, Color::WHITE),
                    text("Open Mobile QR & APK Downloader")
                        .size(12)
                        .font(FONT_MONO_BOLD)
                        .color(Color::WHITE),
                ]
                .spacing(8)
                .align_y(alignment::Vertical::Center),
            )
            .padding([10, 18])
            .style(brand_button)
            .on_press(ModalMsg::OpenMobileFromSettings),
        ]
        .spacing(12)
        .align_x(alignment::Horizontal::Center),
    )
    .width(Length::Fill)
    .padding(28)
    .align_x(alignment::Horizontal::Center)
    .style(dark_card)
    .into()
}

fn view_settings_updates<'a>(st: &'a SettingsModalState) -> Element<'a, ModalMsg> {
    let check_btn = button(
        row![
            icons::refresh_cw(14.0, Color::WHITE),
            text(if st.checking_update {
                "Checking GitHub..."
            } else {
                "Check for Updates"
            })
            .size(12)
            .font(FONT_MONO_BOLD)
            .color(Color::WHITE),
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center),
    )
    .padding([9, 16])
    .style(brand_button);
    let check_btn = if st.checking_update {
        check_btn
    } else {
        check_btn.on_press(ModalMsg::CheckUpdates)
    };

    let mut col = column![
        container(
            row![
                column![
                    text(format!("Glyph Desktop v{APP_VERSION}"))
                        .size(15)
                        .font(FONT_MONO_BOLD)
                        .color(TEXT_PRIMARY),
                    text("Made by TheLunatic1 (Salman Toha) · Pure Rust v3")
                        .size(11)
                        .color(TEXT_SECONDARY),
                ]
                .spacing(3),
                space().width(Length::Fill),
                check_btn,
            ]
            .align_y(alignment::Vertical::Center),
        )
        .width(Length::Fill)
        .padding(16)
        .style(dark_card),
    ]
    .spacing(12);

    if let Some(err) = &st.update_error {
        col = col.push(error_banner(err));
    }

    if let Some(rel) = &st.latest_release {
        let mut release_card = column![
            row![
                text(if st.is_newer_available {
                    format!("New Release Available: {}", rel.tag_name)
                } else {
                    format!("Up to date (Latest GitHub Release: {})", rel.tag_name)
                })
                .size(13)
                .font(FONT_MONO_BOLD)
                .color(if st.is_newer_available {
                    BRAND_300
                } else {
                    GREEN_400
                }),
                space().width(Length::Fill),
                button(
                    row![
                        icons::external_link(13.0, Color::WHITE),
                        text("Open Release").size(11).color(Color::WHITE),
                    ]
                    .spacing(5)
                    .align_y(alignment::Vertical::Center),
                )
                .padding([6, 12])
                .style(brand_button)
                .on_press(ModalMsg::OpenUpdateReleaseUrl(rel.html_url.clone())),
            ]
            .align_y(alignment::Vertical::Center),
        ]
        .spacing(6);

        if let Some(asset) = &rel.asset_name {
            release_card = release_card.push(
                text(format!("Recommended binary: {asset}"))
                    .size(11)
                    .font(FONT_MONO)
                    .color(TEXT_MUTED),
            );
        }

        col = col.push(
            container(release_card)
                .width(Length::Fill)
                .padding(14)
                .style(dark_card),
        );
    }

    col.into()
}

fn view_mobile<'a>(st: &'a MobileModalState) -> Element<'a, ModalMsg> {
    let ver_label = st
        .latest_version
        .clone()
        .unwrap_or_else(|| if st.loading { "Fetching...".into() } else { "Latest".into() });

    let top_bar = container(space().width(Length::Fill).height(4.0))
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(BRAND_500)),
            border: Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });

    let header = row![
        container(icons::smartphone(18.0, BRAND_400))
            .padding(9)
            .style(|_| container::Style {
                background: Some(Background::Color(rgba(BRAND_500, 0.14))),
                border: Border {
                    color: rgba(BRAND_500, 0.3),
                    width: 1.0,
                    radius: 10.0.into(),
                },
                ..Default::default()
            }),
        column![
            row![
                text("Glyph Mobile")
                    .size(18)
                    .font(FONT_MONO_BOLD)
                    .color(TEXT_PRIMARY),
                container(text("Android APK").size(10).font(FONT_MONO_BOLD).color(BRAND_300))
                    .padding([2, 8])
                    .style(pill_badge(rgba(BRAND_500, 0.14), rgba(BRAND_500, 0.3), BRAND_300)),
                container(text(ver_label).size(10).font(FONT_MONO_BOLD).color(GREEN_400))
                    .padding([2, 8])
                    .style(pill_badge(rgba(GREEN_500, 0.14), rgba(GREEN_500, 0.3), GREEN_400)),
            ]
            .spacing(8)
            .align_y(alignment::Vertical::Center),
            text("Manage your infrastructure anywhere — TheLunatic1/glyph-app")
                .size(11)
                .color(TEXT_SECONDARY),
        ]
        .spacing(3),
        space().width(Length::Fill),
        button(icons::x_close(16.0, TEXT_SECONDARY))
            .padding(6)
            .style(ghost_icon_button)
            .on_press(ModalMsg::Close),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center);

    let qr_card: Element<'a, ModalMsg> = if let Some(handle) = &st.qr_handle {
        container(image(handle.clone()).width(216).height(216))
            .padding(10)
            .style(|_| container::Style {
                background: Some(Background::Color(Color::WHITE)),
                border: Border {
                    color: DARK_700,
                    width: 1.0,
                    radius: 12.0.into(),
                },
                ..Default::default()
            })
            .into()
    } else {
        container(icons::qr_code(80.0, TEXT_MUTED))
            .width(Length::Fixed(236.0))
            .height(Length::Fixed(236.0))
            .center_x(Length::Fixed(236.0))
            .center_y(Length::Fixed(236.0))
            .style(dark_card)
            .into()
    };

    let is_apk = st.qr_mode == MobileQrMode::DirectApk;
    let mode_switch = row![
        button(
            text("Direct APK")
                .size(11)
                .font(FONT_MONO_BOLD)
                .color(if is_apk { Color::WHITE } else { TEXT_SECONDARY }),
        )
        .padding([6, 12])
        .style(move |_, _| {
            if is_apk {
                brand_button(&crate::theme::glyph_theme(), button::Status::Active)
            } else {
                secondary_button(&crate::theme::glyph_theme(), button::Status::Active)
            }
        })
        .on_press(ModalMsg::SelectMobileQrMode(MobileQrMode::DirectApk)),
        button(
            text("GitHub Page")
                .size(11)
                .font(FONT_MONO_BOLD)
                .color(if !is_apk { Color::WHITE } else { TEXT_SECONDARY }),
        )
        .padding([6, 12])
        .style(move |_, _| {
            if !is_apk {
                brand_button(&crate::theme::glyph_theme(), button::Status::Active)
            } else {
                secondary_button(&crate::theme::glyph_theme(), button::Status::Active)
            }
        })
        .on_press(ModalMsg::SelectMobileQrMode(MobileQrMode::GithubPage)),
    ]
    .spacing(8);

    let left_col = column![
        qr_card,
        mode_switch,
        text("Scan with Android camera to install")
            .size(11)
            .color(TEXT_MUTED),
    ]
    .spacing(10)
    .align_x(alignment::Horizontal::Center);

    let feat_box = |icon_el: Element<'a, ModalMsg>, title: &'static str, desc: &'static str| {
        container(
            column![
                row![
                    icon_el,
                    text(title)
                        .size(12)
                        .font(FONT_MONO_BOLD)
                        .color(TEXT_PRIMARY),
                ]
                .spacing(6)
                .align_y(alignment::Vertical::Center),
                text(desc).size(11).color(TEXT_SECONDARY),
            ]
            .spacing(4),
        )
        .width(Length::FillPortion(1))
        .padding(10)
        .style(dark_card)
    };

    let features = column![
        row![
            feat_box(
                icons::activity(14.0, GREEN_400).into(),
                "5-Metric Telemetry",
                "Live CPU, RAM, Disk, Network & GPU",
            ),
            feat_box(
                icons::terminal(14.0, BRAND_400).into(),
                "PTY Terminal",
                "Full color interactive mobile shell",
            ),
        ]
        .spacing(8),
        row![
            feat_box(
                icons::folder_open(14.0, ORANGE_400).into(),
                "SFTP & Editor",
                "Browse, upload & edit remote files",
            ),
            feat_box(
                icons::cube(14.0, PURPLE_400).into(),
                "Docker Manager",
                "Inspect, logs & restart containers",
            ),
        ]
        .spacing(8),
    ]
    .spacing(8);

    let sync_banner = container(
        row![
            icons::shield_check(16.0, BRAND_300),
            text("100% Vault Sync: Export .glyph backups on Desktop and import directly into Glyph Android using AES-256-GCM + scrypt.")
                .size(11)
                .color(TEXT_LIGHT),
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center),
    )
    .width(Length::Fill)
    .padding(10)
    .style(|_| container::Style {
        background: Some(Background::Color(rgba(BRAND_500, 0.1))),
        border: Border {
            color: rgba(BRAND_500, 0.3),
            width: 1.0,
            radius: 10.0.into(),
        },
        ..Default::default()
    });

    let right_col = column![
        features,
        sync_banner,
        row![
            button(
                row![
                    if st.copied {
                        icons::check(13.0, GREEN_400)
                    } else {
                        icons::copy(13.0, TEXT_LIGHT)
                    },
                    text(if st.copied { "Copied URL" } else { "Copy URL" })
                        .size(12)
                        .color(if st.copied { GREEN_400 } else { TEXT_LIGHT }),
                ]
                .spacing(6)
                .align_y(alignment::Vertical::Center),
            )
            .padding([9, 14])
            .style(secondary_button)
            .on_press(ModalMsg::CopyMobileUrl),
            button(
                row![
                    icons::download(13.0, Color::WHITE),
                    text(if st.direct_apk_url.is_some() && is_apk {
                        "Download APK"
                    } else {
                        "Open Release Page"
                    })
                    .size(12)
                    .font(FONT_MONO_BOLD)
                    .color(Color::WHITE),
                ]
                .spacing(6)
                .align_y(alignment::Vertical::Center),
            )
            .padding([9, 16])
            .style(brand_button)
            .on_press(ModalMsg::OpenMobileUrl),
        ]
        .spacing(8),
    ]
    .spacing(12)
    .width(Length::Fill);

    let mut content = column![
        top_bar,
        header,
        row![left_col, right_col]
            .spacing(20)
            .align_y(alignment::Vertical::Top),
    ]
    .spacing(14);

    if let Some(err) = &st.error {
        content = content.push(error_banner(err));
    }

    container(content)
        .width(Length::Fixed(720.0))
        .padding(24)
        .style(modal_card)
        .into()
}

fn error_banner<'a, M: 'a>(msg: &'a str) -> Element<'a, M> {
    container(
        row![
            icons::alert_triangle(15.0, RED_400),
            text(msg).size(12).color(RED_400),
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center),
    )
    .width(Length::Fill)
    .padding([9, 12])
    .style(|_| container::Style {
        background: Some(Background::Color(rgba(RED_500, 0.14))),
        border: Border {
            color: rgba(RED_500, 0.35),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    })
    .into()
}

fn success_banner<'a, M: 'a>(msg: &'a str) -> Element<'a, M> {
    container(
        row![
            icons::check(15.0, GREEN_400),
            text(msg).size(12).color(GREEN_400),
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center),
    )
    .width(Length::Fill)
    .padding([9, 12])
    .style(|_| container::Style {
        background: Some(Background::Color(rgba(GREEN_500, 0.14))),
        border: Border {
            color: rgba(GREEN_500, 0.35),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    })
    .into()
}

#[allow(dead_code)]
fn _keep_unused_constants_alive() {
    let _ = (DARK_600, BRAND_300);
}
