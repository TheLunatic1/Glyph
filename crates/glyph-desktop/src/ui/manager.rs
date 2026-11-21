use crate::theme::{
    brand_button, danger_ghost_button, dark_900_bg, dark_card, dark_input, dark_pick_list,
    dark_scrollable, ghost_icon_button, glass_panel, grid_card, logo_handle_for, mobile_pill_button,
    pill_badge, rgba, secondary_button, truncate_ellipsis, APP_VERSION, BRAND_300, BRAND_400,
    BRAND_500, DARK_600, DARK_700, DARK_800, DARK_900, FONT_MONO_MEDIUM, FONT_UI_BOLD,
    FONT_UI_SEMIBOLD, GREEN_400, GREEN_500, RED_400, RED_500, RED_600, TEXT_DIM, TEXT_LIGHT,
    TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY, YELLOW_400, YELLOW_500,
};
use crate::ui::icons;
use crate::ui::os_logo::{os_display_name, os_logo};
use glyph_core::models::{
    format_env_vars, format_port_knock, parse_env_vars, parse_port_knock, AuthMethod, HostGroup,
    IdentitySummary, ServerConfigInput, ServerSummary, SshKeySummary,
};
use iced::widget::{
    button, checkbox, column, container, image, pick_list, row, scrollable, space, text,
    text_input,
};
use iced::{alignment, Background, Border, Color, Element, Length};
use std::collections::{BTreeSet, HashSet};
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupPick {
    pub id: Option<String>,
    pub label: String,
}

impl fmt::Display for GroupPick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityPick {
    pub id: Option<String>,
    pub label: String,
}

impl fmt::Display for IdentityPick {
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
    pub group_id: Option<String>,
    pub identity_id: Option<String>,
    pub tags: String,
    pub ssh_key_id: Option<String>,
    pub private_key_path: String,
    pub passphrase: String,
    pub show_passphrase: bool,
    pub zerotier: String,
    pub proxy_jump_id: Option<String>,
    pub forward_agent: bool,
    pub auto_log: bool,
    pub keepalive_secs: String,
    pub startup_command: String,
    pub env_vars: String,
    pub port_knock: String,
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
            group_id: None,
            identity_id: None,
            tags: String::new(),
            ssh_key_id: None,
            private_key_path: String::new(),
            passphrase: String::new(),
            show_passphrase: false,
            zerotier: String::new(),
            proxy_jump_id: None,
            forward_agent: false,
            auto_log: false,
            keepalive_secs: "30".to_string(),
            startup_command: String::new(),
            env_vars: String::new(),
            port_knock: String::new(),
            show_advanced: false,
            error: None,
        }
    }
}

impl ServerFormState {
    pub fn from_summary(s: &ServerSummary) -> Self {
        let env_str = format_env_vars(&s.env_vars);
        let knock_str = format_port_knock(&s.port_knock);
        let has_adv = s.ssh_key_id.is_some()
            || !s.private_key_path.is_empty()
            || !s.zerotier.is_empty()
            || s.proxy_jump_id.is_some()
            || s.auth_method != AuthMethod::Password
            || s.forward_agent
            || s.auto_log
            || !s.startup_command.is_empty()
            || !env_str.is_empty()
            || !knock_str.is_empty();
        Self {
            editing_id: Some(s.id.clone()),
            name: s.name.clone(),
            host: s.host.clone(),
            port: s.port.to_string(),
            username: s.username.clone(),
            password: String::new(),
            show_password: false,
            auth_method: s.auth_method,
            group_id: s.group_id.clone(),
            identity_id: s.identity_id.clone(),
            tags: s.tags.join(", "),
            ssh_key_id: s.ssh_key_id.clone(),
            private_key_path: s.private_key_path.clone(),
            passphrase: String::new(),
            show_passphrase: false,
            zerotier: s.zerotier.clone(),
            proxy_jump_id: s.proxy_jump_id.clone(),
            forward_agent: s.forward_agent,
            auto_log: s.auto_log,
            keepalive_secs: s.keepalive_secs.to_string(),
            startup_command: s.startup_command.clone(),
            env_vars: env_str,
            port_knock: knock_str,
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
        if username.is_empty() && self.identity_id.is_none() && self.group_id.is_none() {
            return Err("Username is required unless an Identity or Group supplies it.".to_string());
        }
        let tags: Vec<String> = self
            .tags
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
            .collect();
        let keepalive_secs: u16 = self.keepalive_secs.trim().parse().unwrap_or(30);

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
            tags,
            group_id: self.group_id.clone(),
            identity_id: self.identity_id.clone(),
            forward_agent: self.forward_agent,
            keepalive_secs,
            startup_command: self.startup_command.trim().to_string(),
            env_vars: parse_env_vars(&self.env_vars),
            port_knock: parse_port_knock(&self.port_knock),
            saved_tunnels: Vec::new(),
            auto_log: self.auto_log,
        })
    }
}

#[derive(Debug, Clone)]
pub enum ManagerMsg {
    DismissDisconnectBanner,
    SearchChanged(String),
    OpenPalette,
    LockVaultNow,
    OpenMobileModal,
    OpenSshKeysModal,
    OpenIdentitiesModal,
    OpenImportHostsModal,
    OpenImportModal,
    OpenExportModal,
    OpenSettingsModal,

    SelectGroupFilter(Option<String>),
    SelectTagFilter(Option<String>),
    OpenNewGroupModal,
    OpenEditGroupModal(String),

    OpenLocalTerminal,
    OpenSerialConsole,
    OpenTelnetConsole,

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
    FormSelectGroup(GroupPick),
    FormSelectIdentity(IdentityPick),
    FormTagsChanged(String),
    FormToggleAdvanced,
    FormSelectVaultKey(VaultKeyPick),
    FormPrivateKeyPathChanged(String),
    FormBrowsePrivateKey,
    FormPassphraseChanged(String),
    FormToggleShowPassphrase,
    FormZerotierChanged(String),
    FormSelectProxyJump(ProxyJumpPick),
    FormForwardAgentToggled(bool),
    FormAutoLogToggled(bool),
    FormKeepaliveChanged(String),
    FormStartupCommandChanged(String),
    FormEnvVarsChanged(String),
    FormPortKnockChanged(String),
    SubmitForm,
}

fn is_in_group_subtree(
    groups: &[HostGroup],
    server: &ServerSummary,
    target_gid: &str,
) -> bool {
    let mut cur = server.group_id.as_deref();
    let mut seen = HashSet::new();
    while let Some(gid) = cur {
        if gid == target_gid {
            return true;
        }
        if !seen.insert(gid) {
            break;
        }
        cur = groups
            .iter()
            .find(|g| g.id == gid)
            .and_then(|g| g.parent_id.as_deref());
    }
    false
}

fn flatten_group_tree(groups: &[HostGroup]) -> Vec<(&HostGroup, usize)> {
    fn walk<'a>(
        groups: &'a [HostGroup],
        parent_id: Option<&str>,
        depth: usize,
        seen: &mut HashSet<&'a str>,
        out: &mut Vec<(&'a HostGroup, usize)>,
    ) {
        if depth > 12 {
            return;
        }
        for g in groups {
            if g.parent_id.as_deref() == parent_id && seen.insert(g.id.as_str()) {
                out.push((g, depth));
                walk(groups, Some(g.id.as_str()), depth + 1, seen, out);
            }
        }
    }
    let mut out = Vec::with_capacity(groups.len());
    let mut seen = HashSet::new();
    walk(groups, None, 0, &mut seen, &mut out);
    for g in groups {
        if seen.insert(g.id.as_str()) {
            out.push((g, 0));
        }
    }
    out
}

fn collect_unique_tags(servers: &[ServerSummary], groups: &[HostGroup]) -> Vec<String> {
    let mut set = BTreeSet::new();
    for s in servers {
        for t in &s.tags {
            let trimmed = t.trim();
            if !trimmed.is_empty() {
                set.insert(trimmed.to_string());
            }
        }
    }
    for g in groups {
        for t in &g.tags {
            let trimmed = t.trim();
            if !trimmed.is_empty() {
                set.insert(trimmed.to_string());
            }
        }
    }
    set.into_iter().collect()
}

#[allow(clippy::too_many_arguments)]
pub fn view<'a>(
    servers: &'a [ServerSummary],
    groups: &'a [HostGroup],
    identities: &'a [IdentitySummary],
    ssh_keys: &'a [SshKeySummary],
    form: Option<&'a ServerFormState>,
    search_query: &'a str,
    selected_group: Option<&'a str>,
    selected_tag: Option<&'a str>,
    app_lock_enabled: bool,
    disconnect_banner: Option<&'a str>,
    window_width: f32,
) -> Element<'a, ManagerMsg> {
    let mut root = column![].spacing(18);

    if let Some(reason) = disconnect_banner {
        let banner = container(
            row![
                icons::alert_triangle(18.0, Color::WHITE),
                text(format!("Connection Closed: {reason}"))
                    .size(13)
                    .font(FONT_UI_BOLD)
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
            snap: true,
            ..Default::default()
        });
        root = root.push(banner);
    }

    // ── Header + Quick Actions ────────────────────────────────────────────────
    let mut header_right = row![
        button(
            row![
                icons::command(13.0, BRAND_300),
                text("Ctrl+K").size(12).font(FONT_MONO_MEDIUM).color(BRAND_300),
            ]
            .spacing(5)
            .align_y(alignment::Vertical::Center),
        )
        .padding([8, 11])
        .style(secondary_button)
        .on_press(ManagerMsg::OpenPalette),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center);

    if app_lock_enabled {
        header_right = header_right.push(
            button(
                row![
                    icons::lock_open(13.0, TEXT_LIGHT),
                    text("Lock").size(12).color(TEXT_LIGHT),
                ]
                .spacing(5)
                .align_y(alignment::Vertical::Center),
            )
            .padding([8, 11])
            .style(secondary_button)
            .on_press(ManagerMsg::LockVaultNow),
        );
    }

    header_right = header_right
        .push(
            button(
                row![
                    icons::smartphone(14.0, BRAND_300),
                    text("Mobile App").size(12).color(BRAND_300),
                    container(
                        text("APK")
                            .size(10)
                            .font(FONT_UI_BOLD)
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
        )
        .push(
            button(icons::settings(15.0, TEXT_LIGHT))
                .padding([8, 10])
                .style(secondary_button)
                .on_press(ManagerMsg::OpenSettingsModal),
        );

    let brand_header = row![
        container(
            image(logo_handle_for(44))
                .width(44)
                .height(44)
                .filter_method(image::FilterMethod::Linear),
        )
        .padding(4)
        .style(|_| container::Style {
            background: Some(Background::Color(DARK_800)),
            border: Border {
                color: DARK_700,
                width: 1.0,
                radius: 12.0.into(),
            },
            snap: true,
            ..Default::default()
        }),
        column![
            row![
                text("Glyph")
                    .size(28)
                    .font(FONT_UI_BOLD)
                    .color(TEXT_PRIMARY),
                container(
                    text(format!("v{APP_VERSION}"))
                        .size(11)
                        .font(FONT_MONO_MEDIUM)
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
        space().width(Length::Fill),
        header_right,
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center);

    root = root.push(brand_header);

    // ── Left Sidebar: Quick Launch, Groups Tree, Tags ─────────────────────────
    let quick_box = column![
        text("QUICK LAUNCH")
            .size(11)
            .font(FONT_UI_BOLD)
            .color(TEXT_MUTED),
        button(
            row![
                icons::command(14.0, BRAND_400),
                text("Local Terminal").size(12).color(TEXT_LIGHT),
            ]
            .spacing(8)
            .align_y(alignment::Vertical::Center),
        )
        .padding([7, 10])
        .width(Length::Fill)
        .style(secondary_button)
        .on_press(ManagerMsg::OpenLocalTerminal),
        button(
            row![
                icons::cable(14.0, BRAND_400),
                text("Serial Console").size(12).color(TEXT_LIGHT),
            ]
            .spacing(8)
            .align_y(alignment::Vertical::Center),
        )
        .padding([7, 10])
        .width(Length::Fill)
        .style(secondary_button)
        .on_press(ManagerMsg::OpenSerialConsole),
        button(
            row![
                icons::globe(14.0, BRAND_400),
                text("Telnet").size(12).color(TEXT_LIGHT),
            ]
            .spacing(8)
            .align_y(alignment::Vertical::Center),
        )
        .padding([7, 10])
        .width(Length::Fill)
        .style(secondary_button)
        .on_press(ManagerMsg::OpenTelnetConsole),
    ]
    .spacing(6);

    let flat_groups = flatten_group_tree(groups);
    let mut group_col = column![
        row![
            icons::folder_tree(13.0, BRAND_400),
            text("GROUPS")
                .size(11)
                .font(FONT_UI_BOLD)
                .color(TEXT_MUTED)
                .width(Length::Fill),
            button(text("+ Group").size(11).color(BRAND_300))
                .padding([3, 7])
                .style(secondary_button)
                .on_press(ManagerMsg::OpenNewGroupModal),
        ]
        .spacing(6)
        .align_y(alignment::Vertical::Center),
        button(
            text(format!("All Hosts ({})", servers.len()))
                .size(12)
                .font(FONT_UI_SEMIBOLD)
                .color(if selected_group.is_none() {
                    Color::WHITE
                } else {
                    TEXT_LIGHT
                }),
        )
        .padding([7, 10])
        .width(Length::Fill)
        .style(if selected_group.is_none() {
            brand_button
        } else {
            secondary_button
        })
        .on_press(ManagerMsg::SelectGroupFilter(None)),
    ]
    .spacing(5);

    for (grp, depth) in &flat_groups {
        let is_sel = selected_group == Some(grp.id.as_str());
        let gid = grp.id.clone();
        let pad_left = 8 + (*depth as u16) * 12;
        group_col = group_col.push(
            row![
                button(
                    container(
                        text(truncate_ellipsis(&grp.name, 16))
                            .size(12)
                            .color(if is_sel { Color::WHITE } else { TEXT_LIGHT })
                            .wrapping(text::Wrapping::None),
                    )
                    .padding([0, pad_left])
                    .width(Length::Fill),
                )
                .padding([6, 4])
                .width(Length::Fill)
                .style(if is_sel {
                    brand_button
                } else {
                    ghost_icon_button
                })
                .on_press(ManagerMsg::SelectGroupFilter(Some(gid.clone()))),
                button(icons::edit(12.0, TEXT_SECONDARY))
                    .padding(5)
                    .style(ghost_icon_button)
                    .on_press(ManagerMsg::OpenEditGroupModal(gid)),
            ]
            .spacing(2)
            .align_y(alignment::Vertical::Center),
        );
    }

    let tags = collect_unique_tags(servers, groups);
    let mut tags_col = column![
        row![
            icons::tag(13.0, BRAND_400),
            text("TAGS")
                .size(11)
                .font(FONT_UI_BOLD)
                .color(TEXT_MUTED)
                .width(Length::Fill),
        ]
        .spacing(6)
        .align_y(alignment::Vertical::Center),
    ]
    .spacing(5);

    if tags.is_empty() {
        tags_col = tags_col.push(text("No tags yet").size(11).color(TEXT_MUTED));
    } else {
        tags_col = tags_col.push(
            button(
                text("All Tags")
                    .size(11)
                    .color(if selected_tag.is_none() {
                        BRAND_300
                    } else {
                        TEXT_SECONDARY
                    }),
            )
            .padding([4, 8])
            .style(secondary_button)
            .on_press(ManagerMsg::SelectTagFilter(None)),
        );
        for t in tags {
            let active = selected_tag == Some(t.as_str());
            tags_col = tags_col.push(
                button(
                    text(format!("#{t}"))
                        .size(11)
                        .color(if active { Color::WHITE } else { TEXT_LIGHT }),
                )
                .padding([4, 8])
                .width(Length::Fill)
                .style(if active {
                    brand_button
                } else {
                    ghost_icon_button
                })
                .on_press(ManagerMsg::SelectTagFilter(Some(t))),
            );
        }
    }

    let sidebar_panel = container(
        column![quick_box, group_col, tags_col]
            .spacing(16)
            .width(Length::Fill),
    )
    .width(Length::Fixed(224.0))
    .padding(14)
    .style(glass_panel);

    // ── Right Main Area: Search + Action Bar + Form + Grid ────────────────────
    let key_label = if ssh_keys.is_empty() {
        "SSH Keys".to_string()
    } else {
        format!("SSH Keys ({})", ssh_keys.len())
    };
    let id_label = if identities.is_empty() {
        "Identities".to_string()
    } else {
        format!("Identities ({})", identities.len())
    };

    let search_bar = text_input("Search hosts by name, IP, username, or tag...", search_query)
        .on_input(ManagerMsg::SearchChanged)
        .padding([8, 12])
        .size(13)
        .width(Length::Fill)
        .style(dark_input);

    let action_pills = scrollable(
        row![
            button(
                row![
                    icons::users(14.0, TEXT_LIGHT),
                    text(id_label).size(12).color(TEXT_LIGHT),
                ]
                .spacing(6)
                .align_y(alignment::Vertical::Center),
            )
            .padding([8, 11])
            .style(secondary_button)
            .on_press(ManagerMsg::OpenIdentitiesModal),
            button(
                row![
                    icons::key(14.0, TEXT_LIGHT),
                    text(key_label).size(12).color(TEXT_LIGHT),
                ]
                .spacing(6)
                .align_y(alignment::Vertical::Center),
            )
            .padding([8, 11])
            .style(secondary_button)
            .on_press(ManagerMsg::OpenSshKeysModal),
            button(
                row![
                    icons::cloud_download(14.0, BRAND_300),
                    text("Import Hosts").size(12).color(BRAND_300),
                ]
                .spacing(6)
                .align_y(alignment::Vertical::Center),
            )
            .padding([8, 11])
            .style(secondary_button)
            .on_press(ManagerMsg::OpenImportHostsModal),
            button(
                row![
                    icons::upload(14.0, TEXT_LIGHT),
                    text("Import .glyph").size(12).color(TEXT_LIGHT),
                ]
                .spacing(6)
                .align_y(alignment::Vertical::Center),
            )
            .padding([8, 11])
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
            .padding([8, 11])
            .style(secondary_button)
            .on_press(ManagerMsg::OpenExportModal),
            button(
                row![
                    icons::plus(15.0, Color::WHITE),
                    text("Add Server")
                        .size(13)
                        .font(FONT_UI_SEMIBOLD)
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
        .align_y(alignment::Vertical::Center),
    )
    .horizontal()
    .width(Length::Fill)
    .style(dark_scrollable);

    let q_lower = search_query.trim().to_lowercase();
    let filtered: Vec<&ServerSummary> = servers
        .iter()
        .filter(|s| {
            if let Some(gid) = selected_group {
                if !is_in_group_subtree(groups, s, gid) {
                    return false;
                }
            }
            if let Some(tag) = selected_tag {
                let has_tag = s.tags.iter().any(|t| t.eq_ignore_ascii_case(tag));
                if !has_tag {
                    return false;
                }
            }
            if !q_lower.is_empty() {
                let hay = format!(
                    "{} {} {} {}",
                    s.name.to_lowercase(),
                    s.host.to_lowercase(),
                    s.username.to_lowercase(),
                    s.tags.join(" ").to_lowercase()
                );
                if !hay.contains(&q_lower) {
                    return false;
                }
            }
            true
        })
        .collect();

    let mut right_col = column![
        row![
            text(format!("Saved Servers ({})", filtered.len()))
                .size(18)
                .font(FONT_UI_BOLD)
                .color(TEXT_PRIMARY),
            space().width(Length::Fixed(12.0)),
            search_bar,
        ]
        .spacing(12)
        .align_y(alignment::Vertical::Center),
        action_pills,
    ]
    .spacing(14)
    .width(Length::Fill);

    if let Some(f) = form {
        right_col = right_col.push(view_server_form(f, servers, groups, identities, ssh_keys));
    }

    let cols = if window_width < 1040.0 {
        1
    } else if window_width < 1360.0 {
        2
    } else {
        3
    };
    if filtered.is_empty() && form.is_none() {
        let empty = container(
            column![
                icons::server(44.0, TEXT_DIM),
                text(if servers.is_empty() {
                    "No saved servers yet"
                } else {
                    "No hosts match the active filters"
                })
                .size(16)
                .font(FONT_UI_BOLD)
                .color(TEXT_PRIMARY),
                text(if servers.is_empty() {
                    "Add a server or import ~/.ssh/config, PuTTY, CSV, or Cloud instances."
                } else {
                    "Clear your search or choose All Hosts in the left sidebar."
                })
                .size(13)
                .color(TEXT_SECONDARY),
                button(
                    row![
                        icons::plus(15.0, Color::WHITE),
                        text("Add Server")
                            .size(13)
                            .font(FONT_UI_SEMIBOLD)
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
            background: Some(Background::Color(DARK_800)),
            border: Border {
                color: DARK_700,
                width: 1.0,
                radius: 14.0.into(),
            },
            snap: true,
            ..Default::default()
        });
        right_col = right_col.push(empty);
    } else if !filtered.is_empty() {
        let mut grid = column![].spacing(14);
        for chunk in filtered.chunks(cols) {
            let mut r = row![].spacing(14).width(Length::Fill);
            for srv in chunk {
                r = r.push(view_server_card(srv, groups));
            }
            for _ in chunk.len()..cols {
                r = r.push(space().width(Length::Fill));
            }
            grid = grid.push(r);
        }
        right_col = right_col.push(grid);
    }

    if window_width >= 820.0 {
        root = root.push(
            row![sidebar_panel, right_col]
                .spacing(18)
                .align_y(alignment::Vertical::Top),
        );
    } else {
        root = root.push(right_col);
    }

    let inner = container(root)
        .width(Length::Fill)
        .max_width(1360)
        .padding([24, 28]);

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
    groups: &'a [HostGroup],
    identities: &'a [IdentitySummary],
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

    let mut group_options = vec![GroupPick {
        id: None,
        label: "(No Group / Root)".to_string(),
    }];
    for g in groups {
        group_options.push(GroupPick {
            id: Some(g.id.clone()),
            label: g.name.clone(),
        });
    }
    let selected_group = group_options
        .iter()
        .find(|o| o.id == f.group_id)
        .cloned()
        .or_else(|| group_options.first().cloned());

    let mut identity_options = vec![IdentityPick {
        id: None,
        label: "(None — Use host credentials)".to_string(),
    }];
    for ident in identities {
        identity_options.push(IdentityPick {
            id: Some(ident.id.clone()),
            label: format!("{} ({})", ident.name, ident.username),
        });
    }
    let selected_identity = identity_options
        .iter()
        .find(|o| o.id == f.identity_id)
        .cloned()
        .or_else(|| identity_options.first().cloned());

    let mut form_col = column![
        text(title).size(15).font(FONT_UI_BOLD).color(BRAND_300),
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
                    text("Host Group").size(11).color(TEXT_SECONDARY),
                    pick_list(group_options, selected_group, ManagerMsg::FormSelectGroup)
                        .width(Length::Fill)
                        .padding([8, 10])
                        .text_size(12)
                        .style(dark_pick_list),
                ]
                .spacing(5),
            )
            .width(Length::FillPortion(2)),
            container(
                column![
                    text("Reusable Identity").size(11).color(TEXT_SECONDARY),
                    pick_list(
                        identity_options,
                        selected_identity,
                        ManagerMsg::FormSelectIdentity,
                    )
                    .width(Length::Fill)
                    .padding([8, 10])
                    .text_size(12)
                    .style(dark_pick_list),
                ]
                .spacing(5),
            )
            .width(Length::FillPortion(2)),
            container(labeled_input(
                "Tags (comma-separated)",
                "prod, eu-west, db",
                &f.tags,
                false,
                ManagerMsg::FormTagsChanged,
            ))
            .width(Length::FillPortion(2)),
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
                row![
                    labeled_input(
                        "Startup Command (auto-run on shell open)",
                        "e.g. tmux attach || bash",
                        &f.startup_command,
                        false,
                        ManagerMsg::FormStartupCommandChanged,
                    ),
                    container(labeled_input(
                        "Keepalive (secs)",
                        "30",
                        &f.keepalive_secs,
                        false,
                        ManagerMsg::FormKeepaliveChanged,
                    ))
                    .width(Length::Fixed(130.0)),
                ]
                .spacing(12),
                row![
                    labeled_input(
                        "Environment Vars (KEY=VAL, ...)",
                        "e.g. TERM=xterm-256color, EDITOR=vim",
                        &f.env_vars,
                        false,
                        ManagerMsg::FormEnvVarsChanged,
                    ),
                    labeled_input(
                        "Port Knocking (port:proto, ...)",
                        "e.g. 7000:tcp, 8000:udp",
                        &f.port_knock,
                        false,
                        ManagerMsg::FormPortKnockChanged,
                    ),
                ]
                .spacing(12),
                row![
                    checkbox(f.forward_agent).on_toggle(ManagerMsg::FormForwardAgentToggled),
                    text("SSH Agent Forwarding (-A)").size(12).color(TEXT_LIGHT),
                    space().width(16),
                    checkbox(f.auto_log).on_toggle(ManagerMsg::FormAutoLogToggled),
                    text("Auto-Record Session (.cast log)").size(12).color(TEXT_LIGHT),
                ]
                .spacing(8)
                .align_y(alignment::Vertical::Center),
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
                snap: true,
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
                .font(FONT_UI_SEMIBOLD)
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

fn view_server_card<'a>(s: &'a ServerSummary, groups: &'a [HostGroup]) -> Element<'a, ManagerMsg> {
    let raw_title = if s.name.is_empty() { &s.host } else { &s.name };
    let title = truncate_ellipsis(raw_title, 22);
    let subtitle = truncate_ellipsis(&format!("{}@{}:{}", s.username, s.host, s.port), 28);
    let sid_edit = s.id.clone();
    let sid_del = s.id.clone();
    let sid_conn = s.id.clone();

    let top = row![
        os_logo(s.os.as_deref(), raw_title, 40.0),
        container(
            column![
                text(title)
                    .size(15)
                    .font(FONT_UI_BOLD)
                    .color(TEXT_PRIMARY)
                    .wrapping(text::Wrapping::None),
                text(subtitle)
                    .size(12)
                    .font(FONT_MONO_MEDIUM)
                    .color(TEXT_SECONDARY)
                    .wrapping(text::Wrapping::None),
            ]
            .spacing(2)
            .width(Length::Fill),
        )
        .width(Length::Fill)
        .clip(true),
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
    .spacing(10)
    .align_y(alignment::Vertical::Center);

    let os_badge: Element<'a, ManagerMsg> = if let Some(os) = &s.os {
        container(
            text(truncate_ellipsis(&os_display_name(os), 12))
                .size(11)
                .font(FONT_UI_SEMIBOLD)
                .color(GREEN_400)
                .wrapping(text::Wrapping::None),
        )
        .padding([2, 8])
        .style(pill_badge(
            rgba(GREEN_500, 0.14),
            rgba(GREEN_500, 0.3),
            GREEN_400,
        ))
        .into()
    } else {
        container(
            text("Saved")
                .size(11)
                .font(FONT_UI_SEMIBOLD)
                .color(TEXT_SECONDARY)
                .wrapping(text::Wrapping::None),
        )
        .padding([2, 8])
        .style(pill_badge(DARK_900, DARK_700, TEXT_SECONDARY))
        .into()
    };

    let mut badges = row![os_badge].spacing(5).align_y(alignment::Vertical::Center);
    if !s.zerotier.is_empty() {
        badges = badges.push(
            container(text("ZT").size(10).font(FONT_UI_BOLD).color(YELLOW_400))
                .padding([2, 6])
                .style(pill_badge(
                    rgba(YELLOW_500, 0.14),
                    rgba(YELLOW_500, 0.35),
                    YELLOW_400,
                )),
        );
    }
    if s.auth_method == AuthMethod::VaultKey || !s.private_key_path.is_empty() {
        badges = badges.push(
            container(text("KEY").size(10).font(FONT_UI_BOLD).color(BRAND_300))
                .padding([2, 6])
                .style(pill_badge(
                    rgba(BRAND_500, 0.14),
                    rgba(BRAND_500, 0.3),
                    BRAND_300,
                )),
        );
    }
    if s.proxy_jump_id.is_some() {
        badges = badges.push(
            container(text("JUMP").size(10).font(FONT_UI_BOLD).color(BRAND_400))
                .padding([2, 6])
                .style(pill_badge(
                    rgba(BRAND_500, 0.12),
                    rgba(BRAND_500, 0.28),
                    BRAND_400,
                )),
        );
    }
    if s.forward_agent {
        badges = badges.push(
            container(text("-A").size(10).font(FONT_UI_BOLD).color(GREEN_400))
                .padding([2, 6])
                .style(pill_badge(
                    rgba(GREEN_500, 0.12),
                    rgba(GREEN_500, 0.28),
                    GREEN_400,
                )),
        );
    }
    if let Some(gid) = &s.group_id {
        if let Some(grp) = groups.iter().find(|g| &g.id == gid) {
            badges = badges.push(
                container(
                    text(truncate_ellipsis(&grp.name, 10))
                        .size(11)
                        .color(BRAND_300)
                        .wrapping(text::Wrapping::None),
                )
                .padding([2, 6])
                .style(pill_badge(DARK_900, DARK_700, BRAND_300)),
            );
        }
    }
    for t in s.tags.iter().take(2) {
        badges = badges.push(
            container(
                text(format!("#{}", truncate_ellipsis(t, 9)))
                    .size(11)
                    .color(TEXT_SECONDARY)
                    .wrapping(text::Wrapping::None),
            )
            .padding([2, 6])
            .style(pill_badge(DARK_900, DARK_700, TEXT_SECONDARY)),
        );
    }

    let bottom = row![
        container(badges).width(Length::Fill).clip(true),
        button(
            row![
                text("Connect")
                    .size(12)
                    .font(FONT_UI_SEMIBOLD)
                    .color(Color::WHITE)
                    .wrapping(text::Wrapping::None),
                icons::play(11.0, Color::WHITE),
            ]
            .spacing(6)
            .align_y(alignment::Vertical::Center),
        )
        .padding([6, 12])
        .style(brand_button)
        .on_press(ManagerMsg::ConnectServer(sid_conn)),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center);

    container(column![top, bottom].spacing(16))
        .width(Length::Fill)
        .padding(18)
        .style(grid_card)
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
