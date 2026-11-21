use crate::theme::{
    brand_button, danger_button, danger_ghost_button, dark_card, dark_input, dark_pick_list,
    dark_scrollable, ghost_icon_button, modal_backdrop, modal_card, pill_badge, rgba,
    secondary_button, APP_VERSION, BRAND_300, BRAND_400, BRAND_500, DARK_600, DARK_700, DARK_800,
    DARK_900, FONT_MONO, FONT_MONO_BOLD, GREEN_400, GREEN_500, ORANGE_400, RED_400, RED_500,
    TEXT_DIM, TEXT_LIGHT, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY, YELLOW_400, YELLOW_500,
};
use crate::ui::icons;
use crate::ui::os_logo::os_logo;
use glyph_core::models::{
    KnownHostEntry, ServerEntry, ServerSummary, SshKeyAlgorithm, SshKeySummary,
};
use iced::widget::{
    button, checkbox, column, container, mouse_area, pick_list, row, scrollable, space, stack,
    text, text_input,
};
use iced::{alignment, Background, Border, Color, Element, Length};
use std::collections::BTreeSet;
use std::fmt;

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

#[derive(Debug, Clone)]
pub struct SettingsModalState {
    pub encryption_available: bool,
    pub data_dir: String,
}

#[derive(Debug, Clone)]
pub enum ManagerModal {
    DeleteConfirm(DeleteConfirmState),
    Export(ExportModalState),
    Import(ImportModalState),
    SshKeys(SshKeysModalState),
    Connecting(ConnectingModalState),
    Settings(SettingsModalState),
    Mobile,
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
    RemoveKnownHost(String),
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
        ManagerModal::Mobile => view_mobile(),
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
            text("Settings & Security")
                .size(17)
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

    let content = column![
        header,
        vault_card,
        text("Trusted SSH Host Keys (TOFU Known Hosts)")
            .size(12)
            .font(FONT_MONO_BOLD)
            .color(TEXT_LIGHT),
        container(
            scrollable(kh_col)
                .height(Length::Fixed(160.0))
                .style(dark_scrollable)
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
        }),
        text("Milestone 2 Preview: Local REST API settings, Auto-Update checks, and VNC defaults arrive in the next milestone.")
            .size(11)
            .color(TEXT_MUTED),
        row![
            space().width(Length::Fill),
            button(text("Done").size(13).font(FONT_MONO_BOLD).color(Color::WHITE))
                .padding([8, 18])
                .style(brand_button)
                .on_press(ModalMsg::Close),
        ],
    ]
    .spacing(12);

    container(content)
        .width(Length::Fixed(580.0))
        .padding(24)
        .style(modal_card)
        .into()
}

fn view_mobile<'a>() -> Element<'a, ModalMsg> {
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
            text("Glyph Mobile App (Android APK)")
                .size(17)
                .font(FONT_MONO_BOLD)
                .color(TEXT_PRIMARY),
            text("100% Vault Backup Interoperability (.glyph)")
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

    let body = container(
        column![
            icons::qr_code(48.0, BRAND_300),
            text("QR Transfer & Live Pairing Coming in Milestone 2")
                .size(14)
                .font(FONT_MONO_BOLD)
                .color(TEXT_PRIMARY),
            text("You can already export any server list using \"Export (.glyph)\" and import it directly into the existing Glyph Android app — both use identical AES-256-GCM + scrypt (N=16384, r=8, p=1) encryption.")
                .size(12)
                .color(TEXT_SECONDARY),
        ]
        .spacing(10)
        .align_x(alignment::Horizontal::Center),
    )
    .width(Length::Fill)
    .padding(22)
    .align_x(alignment::Horizontal::Center)
    .style(dark_card);

    let content = column![
        header,
        body,
        row![
            space().width(Length::Fill),
            button(text("Close").size(13).color(TEXT_LIGHT))
                .padding([8, 18])
                .style(secondary_button)
                .on_press(ModalMsg::Close),
        ],
    ]
    .spacing(16);

    container(content)
        .width(Length::Fixed(500.0))
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
