use crate::theme::{
    brand_button, danger_ghost_button, dark_900_bg, dark_card, dark_input, dark_pick_list,
    dark_scrollable, ghost_icon_button, glass_panel, mobile_pill_button, pill_badge, rgba,
    secondary_button, APP_VERSION, BRAND_300, BRAND_400, BRAND_500, DARK_600, DARK_700, DARK_800,
    DARK_900, FONT_MONO, FONT_MONO_BOLD, GREEN_400, GREEN_500, LOGO_PNG, RED_400, RED_500,
    RED_600, TEXT_DIM, TEXT_LIGHT, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY, YELLOW_400,
    YELLOW_500,
};
use crate::ui::icons;
use crate::ui::os_logo::os_logo;
use glyph_core::models::{AuthMethod, ServerConfigInput, ServerSummary, SshKeySummary};
use iced::widget::{
    button, column, container, image, pick_list, row, scrollable, space, text, text_input,
};
use iced::{alignment, Background, Border, Color, Element, Length};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultKeyPick {
    pub id: Option<String>,
    pub label: String,
}

impl fmt::Display for VaultKeyPick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyJumpPick {
    pub id: Option<String>,
    pub label: String,
}

impl fmt::Display for ProxyJumpPick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label)
    }
}

#[derive(Debug, Clone)]
pub struct ServerFormState {
    pub editing_id: Option<String>,
    pub name: String,
    pub host: String,
    pub port: String,
    pub username: String,
    pub password: String,
    pub show_password: bool,
    pub auth_method: AuthMethod,
    pub ssh_key_id: Option<String>,
    pub private_key_path: String,
    pub passphrase: String,
    pub show_passphrase: bool,
    pub zerotier: String,
    pub proxy_jump_id: Option<String>,
    pub show_advanced: bool,
    pub error: Option<String>,
}

impl Default for ServerFormState {
    fn default() -> Self {
        Self {
            editing_id: None,
            name: String::new(),
            host: String::new(),
            port: "22".to_string(),
            username: "root".to_string(),
            password: String::new(),
            show_password: false,
            auth_method: AuthMethod::Password,
            ssh_key_id: None,
            private_key_path: String::new(),
            passphrase: String::new(),
            show_passphrase: false,
            zerotier: String::new(),
            proxy_jump_id: None,
            show_advanced: false,
            error: None,
        }
    }
}

impl ServerFormState {
    pub fn from_summary(s: &ServerSummary) -> Self {
        let has_adv = s.ssh_key_id.is_some()
            || !s.private_key_path.is_empty()
            || !s.zerotier.is_empty()
            || s.proxy_jump_id.is_some()
            || s.auth_method != AuthMethod::Password;
        Self {
            editing_id: Some(s.id.clone()),
            name: s.name.clone(),
            host: s.host.clone(),
            port: s.port.to_string(),
            username: s.username.clone(),
            password: String::new(),
            show_password: false,
            auth_method: s.auth_method,
            ssh_key_id: s.ssh_key_id.clone(),
            private_key_path: s.private_key_path.clone(),
            passphrase: String::new(),
            show_passphrase: false,
            zerotier: s.zerotier.clone(),
            proxy_jump_id: s.proxy_jump_id.clone(),
            show_advanced: has_adv,
            error: None,
        }
    }

    pub fn to_input(&self) -> Result<ServerConfigInput, String> {
        let host = self.host.trim().to_string();
        if host.is_empty() {
            return Err("Host / IP address is required.".to_string());
        }
        let port: u16 = self
            .port
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|&p| p > 0)
            .ok_or_else(|| "Port must be a number between 1 and 65535.".to_string())?;
        let username = self.username.trim().to_string();
        if username.is_empty() {
            return Err("Username is required.".to_string());
        }

        Ok(ServerConfigInput {
            id: self.editing_id.clone(),
            name: self.name.trim().to_string(),
            host,
            username,
            port,
            password: if self.password.is_empty() {
                None
            } else {
                Some(self.password.clone())
            },
            zerotier: self.zerotier.trim().to_string(),
            private_key_path: self.private_key_path.trim().to_string(),
            auth_method: self.auth_method,
            ssh_key_id: self.ssh_key_id.clone(),
            passphrase: if self.passphrase.is_empty() {
                None
            } else {
                Some(self.passphrase.clone())
            },
            proxy_jump_id: self.proxy_jump_id.clone(),
            tags: Vec::new(),
        })
    }
}

#[derive(Debug, Clone)]
pub enum ManagerMsg {
    DismissDisconnectBanner,
    OpenMobileModal,
    OpenSshKeysModal,
    OpenImportModal,
    OpenExportModal,
    OpenSettingsModal,

    ToggleAddForm,
    StartEditServer(String),
    RequestDeleteServer(String),
    ConnectServer(String),
    CancelForm,

    FormNameChanged(String),
    FormHostChanged(String),
    FormPortChanged(String),
    FormUsernameChanged(String),
    FormPasswordChanged(String),
    FormToggleShowPassword,
    FormAuthMethodChanged(AuthMethod),
    FormToggleAdvanced,
    FormSelectVaultKey(VaultKeyPick),
    FormPrivateKeyPathChanged(String),
    FormBrowsePrivateKey,
    FormPassphraseChanged(String),
    FormToggleShowPassphrase,
    FormZerotierChanged(String),
    FormSelectProxyJump(ProxyJumpPick),
    SubmitForm,
}

pub fn view<'a>(
    servers: &'a [ServerSummary],
    ssh_keys: &'a [SshKeySummary],
    form: Option<&'a ServerFormState>,
    disconnect_banner: Option<&'a str>,
) -> Element<'a, ManagerMsg> {
    let mut root = column![].spacing(24);

    if let Some(reason) = disconnect_banner {
        let banner = container(
            row![
                icons::alert_triangle(18.0, Color::WHITE),
                text(format!("Connection Closed: {reason}"))
                    .size(13)
                    .font(FONT_MONO_BOLD)
                    .color(Color::WHITE),
                space().width(Length::Fill),
                button(icons::x_close(16.0, Color::WHITE))
                    .padding(4)
                    .style(ghost_icon_button)
                    .on_press(ManagerMsg::DismissDisconnectBanner),
            ]
            .spacing(10)
            .align_y(alignment::Vertical::Center),
        )
        .width(Length::Fill)
        .padding([12, 16])
        .style(|_| container::Style {
            background: Some(Background::Color(RED_600)),
            border: Border {
                color: RED_400,
                width: 1.0,
                radius: 10.0.into(),
            },
            ..Default::default()
        });
        root = root.push(banner);
    }

    // ── Header ────────────────────────────────────────────────────────────────
    let brand_header = row![
        container(
            image(image::Handle::from_bytes(LOGO_PNG))
                .width(44)
                .height(44)
        )
        .padding(4)
        .style(|_| container::Style {
            background: Some(Background::Color(DARK_800)),
            border: Border {
                color: DARK_700,
                width: 1.0,
                radius: 12.0.into(),
            },
            ..Default::default()
        }),
        column![
            row![
                text("Glyph")
                    .size(28)
                    .font(FONT_MONO_BOLD)
                    .color(TEXT_PRIMARY),
                container(
                    text(format!("v{APP_VERSION}"))
                        .size(11)
                        .font(FONT_MONO)
                        .color(BRAND_300)
                )
                .padding([2, 8])
                .style(pill_badge(
                    rgba(BRAND_500, 0.14),
                    rgba(BRAND_500, 0.3),
                    BRAND_300,
                )),
            ]
            .spacing(10)
            .align_y(alignment::Vertical::Center),
            text("Secure SSH & Server Management")
                .size(13)
                .color(TEXT_SECONDARY),
        ]
        .spacing(2),
    ]
    .spacing(14)
    .align_y(alignment::Vertical::Center);

    root = root.push(brand_header);

    // ── Saved Servers Action Bar ──────────────────────────────────────────────
    let key_label = if ssh_keys.is_empty() {
        "SSH Keys".to_string()
    } else {
        format!("SSH Keys ({})", ssh_keys.len())
    };

    let action_bar = row![
        text("Saved Servers")
            .size(18)
            .font(FONT_MONO_BOLD)
            .color(TEXT_PRIMARY),
        space().width(Length::Fill),
        button(
            row![
                icons::smartphone(14.0, BRAND_300),
                text("Mobile App").size(12).color(BRAND_300),
                container(
                    text("APK")
                        .size(9)
                        .font(FONT_MONO_BOLD)
                        .color(BRAND_300)
                )
                .padding([1, 5])
                .style(pill_badge(
                    rgba(BRAND_500, 0.25),
                    rgba(BRAND_500, 0.45),
                    BRAND_300,
                )),
            ]
            .spacing(6)
            .align_y(alignment::Vertical::Center),
        )
        .padding([8, 12])
        .style(mobile_pill_button)
        .on_press(ManagerMsg::OpenMobileModal),
        button(
            row![
                icons::key(14.0, TEXT_LIGHT),
                text(key_label).size(12).color(TEXT_LIGHT),
            ]
            .spacing(6)
            .align_y(alignment::Vertical::Center),
        )
        .padding([8, 12])
        .style(secondary_button)
        .on_press(ManagerMsg::OpenSshKeysModal),
        button(
            row![
                icons::upload(14.0, TEXT_LIGHT),
                text("Import").size(12).color(TEXT_LIGHT),
            ]
            .spacing(6)
            .align_y(alignment::Vertical::Center),
        )
        .padding([8, 12])
        .style(secondary_button)
        .on_press(ManagerMsg::OpenImportModal),
        button(
            row![
                icons::download(14.0, TEXT_LIGHT),
                text("Export").size(12).color(TEXT_LIGHT),
            ]
            .spacing(6)
            .align_y(alignment::Vertical::Center),
        )
        .padding([8, 12])
        .style(secondary_button)
        .on_press(ManagerMsg::OpenExportModal),
        button(icons::settings(15.0, TEXT_LIGHT))
            .padding([8, 10])
            .style(secondary_button)
            .on_press(ManagerMsg::OpenSettingsModal),
        button(
            row![
                icons::plus(15.0, Color::WHITE),
                text("Add Server")
                    .size(13)
                    .font(FONT_MONO_BOLD)
                    .color(Color::WHITE),
            ]
            .spacing(6)
            .align_y(alignment::Vertical::Center),
        )
        .padding([8, 14])
        .style(brand_button)
        .on_press(ManagerMsg::ToggleAddForm),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center);

    root = root.push(action_bar);

    // ── Add / Edit Server Form ────────────────────────────────────────────────
    if let Some(f) = form {
        root = root.push(view_server_form(f, servers, ssh_keys));
    }

    // ── Server Cards Grid (3 columns) ─────────────────────────────────────────
    if servers.is_empty() && form.is_none() {
        let empty = container(
            column![
                icons::server(44.0, TEXT_DIM),
                text("No saved servers yet")
                    .size(16)
                    .font(FONT_MONO_BOLD)
                    .color(TEXT_PRIMARY),
                text("Add your first Linux or macOS server to start managing it via SSH.")
                    .size(13)
                    .color(TEXT_SECONDARY),
                button(
                    row![
                        icons::plus(15.0, Color::WHITE),
                        text("Add Server")
                            .size(13)
                            .font(FONT_MONO_BOLD)
                            .color(Color::WHITE),
                    ]
                    .spacing(6)
                    .align_y(alignment::Vertical::Center),
                )
                .padding([9, 18])
                .style(brand_button)
                .on_press(ManagerMsg::ToggleAddForm),
            ]
            .spacing(10)
            .align_x(alignment::Horizontal::Center),
        )
        .width(Length::Fill)
        .padding(44)
        .align_x(alignment::Horizontal::Center)
        .style(|_| container::Style {
            background: Some(Background::Color(rgba(DARK_800, 0.45))),
            border: Border {
                color: DARK_700,
                width: 1.0,
                radius: 14.0.into(),
            },
            ..Default::default()
        });
        root = root.push(empty);
    } else if !servers.is_empty() {
        let mut grid = column![].spacing(14);
        for chunk in servers.chunks(3) {
            let mut r = row![].spacing(14);
            for srv in chunk {
                r = r.push(view_server_card(srv));
            }
            for _ in chunk.len()..3 {
                r = r.push(space().width(Length::Fill));
            }
            grid = grid.push(r);
        }
        root = root.push(grid);
    }

    let inner = container(root)
        .width(Length::Fill)
        .max_width(1140)
        .padding([28, 32]);

    container(
        scrollable(
            container(inner)
                .width(Length::Fill)
                .align_x(alignment::Horizontal::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(dark_scrollable),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(dark_900_bg)
    .into()
}

fn view_server_form<'a>(
    f: &'a ServerFormState,
    servers: &'a [ServerSummary],
    ssh_keys: &'a [SshKeySummary],
) -> Element<'a, ManagerMsg> {
    let title = if f.editing_id.is_some() {
        "Edit Server Configuration (Secure Vault)"
    } else {
        "Add New Server (Secure Vault)"
    };

    const AUTH_METHODS: [AuthMethod; 4] = [
        AuthMethod::Password,
        AuthMethod::VaultKey,
        AuthMethod::KeyFile,
        AuthMethod::Agent,
    ];

    let mut form_col = column![
        text(title).size(15).font(FONT_MONO_BOLD).color(BRAND_300),
        row![
            labeled_input(
                "Name (Alias)",
                "e.g. Production API",
                &f.name,
                false,
                ManagerMsg::FormNameChanged,
            ),
            labeled_input(
                "Host / IP",
                "e.g. 192.168.1.50 or server.example.com",
                &f.host,
                false,
                ManagerMsg::FormHostChanged,
            ),
        ]
        .spacing(12),
        row![
            container(labeled_input(
                "Port",
                "22",
                &f.port,
                false,
                ManagerMsg::FormPortChanged,
            ))
            .width(Length::FillPortion(1)),
            container(labeled_input(
                "Username",
                "root",
                &f.username,
                false,
                ManagerMsg::FormUsernameChanged,
            ))
            .width(Length::FillPortion(2)),
            container(
                column![
                    text("Password (optional if using SSH Key)")
                        .size(11)
                        .color(TEXT_SECONDARY),
                    row![
                        text_input(
                            if f.editing_id.is_some() {
                                "Leave blank to keep saved password"
                            } else {
                                "Enter SSH password..."
                            },
                            &f.password,
                        )
                        .secure(!f.show_password)
                        .size(13)
                        .padding([8, 10])
                        .style(dark_input)
                        .on_input(ManagerMsg::FormPasswordChanged),
                        button(if f.show_password {
                            icons::eye_off(15.0, TEXT_SECONDARY)
                        } else {
                            icons::eye(15.0, TEXT_SECONDARY)
                        })
                        .padding([7, 9])
                        .style(secondary_button)
                        .on_press(ManagerMsg::FormToggleShowPassword),
                    ]
                    .spacing(6),
                ]
                .spacing(5),
            )
            .width(Length::FillPortion(3)),
        ]
        .spacing(12),
        row![
            container(
                column![
                    text("Authentication Method").size(11).color(TEXT_SECONDARY),
                    pick_list(
                        AUTH_METHODS.as_slice(),
                        Some(f.auth_method),
                        ManagerMsg::FormAuthMethodChanged,
                    )
                    .width(Length::Fill)
                    .padding([8, 10])
                    .text_size(12)
                    .style(dark_pick_list),
                ]
                .spacing(5),
            )
            .width(Length::Fill),
            container(
                column![
                    text("Advanced SSH & Network Options")
                        .size(11)
                        .color(TEXT_SECONDARY),
                    button(
                        row![
                            icons::chevron_down(14.0, TEXT_LIGHT),
                            text(if f.show_advanced {
                                "Hide Advanced Options"
                            } else {
                                "Show Advanced Options (Keys, ZeroTier, ProxyJump)"
                            })
                            .size(12)
                            .color(TEXT_LIGHT),
                        ]
                        .spacing(6)
                        .align_y(alignment::Vertical::Center),
                    )
                    .width(Length::Fill)
                    .padding([9, 12])
                    .style(secondary_button)
                    .on_press(ManagerMsg::FormToggleAdvanced),
                ]
                .spacing(5),
            )
            .width(Length::Fill),
        ]
        .spacing(12),
    ]
    .spacing(12);

    if f.show_advanced || f.auth_method == AuthMethod::VaultKey || f.auth_method == AuthMethod::KeyFile {
        let mut key_options = vec![VaultKeyPick {
            id: None,
            label: "None (Use file path / password / agent)".to_string(),
        }];
        for k in ssh_keys {
            key_options.push(VaultKeyPick {
                id: Some(k.id.clone()),
                label: format!("{} ({})", k.name, k.algorithm),
            });
        }
        let selected_key = key_options
            .iter()
            .find(|o| o.id == f.ssh_key_id)
            .cloned()
            .or_else(|| key_options.first().cloned());

        let mut jump_options = vec![ProxyJumpPick {
            id: None,
            label: "Direct Connection (No Bastion)".to_string(),
        }];
        for s in servers {
            if f.editing_id.as_deref() == Some(s.id.as_str()) {
                continue;
            }
            jump_options.push(ProxyJumpPick {
                id: Some(s.id.clone()),
                label: format!(
                    "{} ({}@{}:{})",
                    if s.name.is_empty() { &s.host } else { &s.name },
                    s.username,
                    s.host,
                    s.port
                ),
            });
        }
        let selected_jump = jump_options
            .iter()
            .find(|o| o.id == f.proxy_jump_id)
            .cloned()
            .or_else(|| jump_options.first().cloned());

        let adv_box = container(
            column![
                row![
                    container(
                        column![
                            text("Vault SSH Key").size(11).color(TEXT_SECONDARY),
                            pick_list(
                                key_options,
                                selected_key,
                                ManagerMsg::FormSelectVaultKey,
                            )
                            .width(Length::Fill)
                            .padding([8, 10])
                            .text_size(12)
                            .style(dark_pick_list),
                        ]
                        .spacing(5),
                    )
                    .width(Length::Fill),
                    container(
                        column![
                            text("SSH Private Key File Path")
                                .size(11)
                                .color(TEXT_SECONDARY),
                            row![
                                text_input(
                                    "Optional ~/.ssh/id_ed25519 path",
                                    &f.private_key_path,
                                )
                                .size(12)
                                .padding([8, 10])
                                .style(dark_input)
                                .on_input(ManagerMsg::FormPrivateKeyPathChanged),
                                button(text("Browse...").size(12).color(TEXT_LIGHT))
                                    .padding([8, 12])
                                    .style(secondary_button)
                                    .on_press(ManagerMsg::FormBrowsePrivateKey),
                            ]
                            .spacing(6),
                        ]
                        .spacing(5),
                    )
                    .width(Length::Fill),
                ]
                .spacing(12),
                row![
                    container(
                        column![
                            text("Key Passphrase (optional)")
                                .size(11)
                                .color(TEXT_SECONDARY),
                            row![
                                text_input(
                                    if f.editing_id.is_some() {
                                        "Leave blank to keep existing"
                                    } else {
                                        "Private key passphrase..."
                                    },
                                    &f.passphrase,
                                )
                                .secure(!f.show_passphrase)
                                .size(12)
                                .padding([8, 10])
                                .style(dark_input)
                                .on_input(ManagerMsg::FormPassphraseChanged),
                                button(if f.show_passphrase {
                                    icons::eye_off(14.0, TEXT_SECONDARY)
                                } else {
                                    icons::eye(14.0, TEXT_SECONDARY)
                                })
                                .padding([7, 9])
                                .style(secondary_button)
                                .on_press(ManagerMsg::FormToggleShowPassphrase),
                            ]
                            .spacing(6),
                        ]
                        .spacing(5),
                    )
                    .width(Length::Fill),
                    container(labeled_input(
                        "ZeroTier Network ID (16 hex chars, optional)",
                        "e.g. 8056c2e21c000001",
                        &f.zerotier,
                        false,
                        ManagerMsg::FormZerotierChanged,
                    ))
                    .width(Length::Fill),
                ]
                .spacing(12),
                column![
                    text("ProxyJump / Bastion Host (optional)")
                        .size(11)
                        .color(TEXT_SECONDARY),
                    pick_list(
                        jump_options,
                        selected_jump,
                        ManagerMsg::FormSelectProxyJump,
                    )
                    .width(Length::Fill)
                    .padding([8, 10])
                    .text_size(12)
                    .style(dark_pick_list),
                ]
                .spacing(5),
            ]
            .spacing(12),
        )
        .width(Length::Fill)
        .padding(14)
        .style(dark_card);

        form_col = form_col.push(adv_box);
    }

    if let Some(err) = &f.error {
        form_col = form_col.push(
            container(
                row![
                    icons::alert_triangle(15.0, RED_400),
                    text(err).size(12).color(RED_400),
                ]
                .spacing(8)
                .align_y(alignment::Vertical::Center),
            )
            .width(Length::Fill)
            .padding([8, 12])
            .style(|_| container::Style {
                background: Some(Background::Color(rgba(RED_500, 0.14))),
                border: Border {
                    color: rgba(RED_500, 0.35),
                    width: 1.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            }),
        );
    }

    form_col = form_col.push(
        row![
            space().width(Length::Fill),
            button(text("Cancel").size(13).color(TEXT_LIGHT))
                .padding([9, 16])
                .style(secondary_button)
                .on_press(ManagerMsg::CancelForm),
            button(
                text(if f.editing_id.is_some() {
                    "Update Server"
                } else {
                    "Save Server"
                })
                .size(13)
                .font(FONT_MONO_BOLD)
                .color(Color::WHITE),
            )
            .padding([9, 18])
            .style(brand_button)
            .on_press(ManagerMsg::SubmitForm),
        ]
        .spacing(10),
    );

    container(form_col)
        .width(Length::Fill)
        .padding(20)
        .style(glass_panel)
        .into()
}

fn view_server_card<'a>(s: &'a ServerSummary) -> Element<'a, ManagerMsg> {
    let title = if s.name.is_empty() { &s.host } else { &s.name };
    let sid_edit = s.id.clone();
    let sid_del = s.id.clone();
    let sid_conn = s.id.clone();

    let top = row![
        os_logo(s.os.as_deref(), title, 40.0),
        column![
            text(title)
                .size(15)
                .font(FONT_MONO_BOLD)
                .color(TEXT_PRIMARY),
            text(format!("{}@{}:{}", s.username, s.host, s.port))
                .size(11)
                .font(FONT_MONO)
                .color(TEXT_SECONDARY),
        ]
        .spacing(2)
        .width(Length::Fill),
        row![
            button(icons::edit(14.0, TEXT_SECONDARY))
                .padding(6)
                .style(ghost_icon_button)
                .on_press(ManagerMsg::StartEditServer(sid_edit)),
            button(icons::trash(14.0, RED_400))
                .padding(6)
                .style(danger_ghost_button)
                .on_press(ManagerMsg::RequestDeleteServer(sid_del)),
        ]
        .spacing(2),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center);

    let os_badge: Element<'a, ManagerMsg> = if let Some(os) = &s.os {
        container(text(os).size(10).font(FONT_MONO_BOLD).color(GREEN_400))
            .padding([2, 8])
            .style(pill_badge(
                rgba(GREEN_500, 0.14),
                rgba(GREEN_500, 0.3),
                GREEN_400,
            ))
            .into()
    } else {
        container(text("Saved").size(10).color(TEXT_SECONDARY))
            .padding([2, 8])
            .style(pill_badge(DARK_900, DARK_700, TEXT_SECONDARY))
            .into()
    };

    let mut badges = row![os_badge].spacing(6);
    if !s.zerotier.is_empty() {
        badges = badges.push(
            container(text("ZT").size(10).font(FONT_MONO_BOLD).color(YELLOW_400))
                .padding([2, 7])
                .style(pill_badge(
                    rgba(YELLOW_500, 0.14),
                    rgba(YELLOW_500, 0.35),
                    YELLOW_400,
                )),
        );
    }
    if s.auth_method == AuthMethod::VaultKey || !s.private_key_path.is_empty() {
        badges = badges.push(
            container(text("KEY").size(10).font(FONT_MONO_BOLD).color(BRAND_300))
                .padding([2, 7])
                .style(pill_badge(
                    rgba(BRAND_500, 0.14),
                    rgba(BRAND_500, 0.3),
                    BRAND_300,
                )),
        );
    }
    if s.proxy_jump_id.is_some() {
        badges = badges.push(
            container(text("JUMP").size(10).font(FONT_MONO_BOLD).color(BRAND_400))
                .padding([2, 7])
                .style(pill_badge(
                    rgba(BRAND_500, 0.12),
                    rgba(BRAND_500, 0.28),
                    BRAND_400,
                )),
        );
    }

    let bottom = row![
        badges,
        space().width(Length::Fill),
        button(
            row![
                text("Connect")
                    .size(12)
                    .font(FONT_MONO_BOLD)
                    .color(Color::WHITE),
                icons::play(11.0, Color::WHITE),
            ]
            .spacing(6)
            .align_y(alignment::Vertical::Center),
        )
        .padding([6, 12])
        .style(brand_button)
        .on_press(ManagerMsg::ConnectServer(sid_conn)),
    ]
    .align_y(alignment::Vertical::Center);

    container(column![top, bottom].spacing(18))
        .width(Length::Fill)
        .padding(18)
        .style(glass_panel)
        .into()
}

fn labeled_input<'a>(
    label: &'a str,
    placeholder: &'a str,
    val: &'a str,
    secure: bool,
    on_input: impl Fn(String) -> ManagerMsg + 'a,
) -> Element<'a, ManagerMsg> {
    column![
        text(label).size(11).color(TEXT_SECONDARY),
        text_input(placeholder, val)
            .secure(secure)
            .size(13)
            .padding([8, 10])
            .style(dark_input)
            .on_input(on_input),
    ]
    .spacing(5)
    .width(Length::Fill)
    .into()
}

#[allow(dead_code)]
fn _keep_unused() {
    let _ = (DARK_600, TEXT_MUTED);
}
