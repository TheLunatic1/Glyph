use crate::theme::{
    brand_button, danger_ghost_button, dark_input, dark_pick_list, dark_scrollable, glass_panel,
    modal_card, rgba, secondary_button, truncate_ellipsis, BRAND_400, BRAND_500, FONT_MONO_BOLD,
    GREEN_400, RED_400, TEXT_LIGHT, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY,
};
use crate::ui::icons;
use glyph_core::cloud_import::CloudProvider;
use glyph_core::importers::default_ssh_config_path;
use glyph_core::models::{
    AuthMethod, HostGroup, IdentitySummary, ServerConfigInput, ServerSummary, SshKeySummary,
};
use iced::widget::{
    button, checkbox, column, container, mouse_area, pick_list, row, scrollable, space, stack,
    text, text_input,
};
use iced::{alignment, window, Background, Border, Color, Element, Length, Theme};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionChoice {
    pub id: String,
    pub label: String,
}

impl std::fmt::Display for OptionChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

// ---------------------------------------------------------------------------
// 1. Identities Modal
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct IdentitiesModalState {
    pub editing_id: Option<String>,
    pub name: String,
    pub username: String,
    pub auth_method: AuthMethod,
    pub password: String,
    pub ssh_key_id: Option<String>,
    pub private_key_path: String,
    pub passphrase: String,
    pub status: Option<String>,
    pub error: Option<String>,
}

impl Default for IdentitiesModalState {
    fn default() -> Self {
        Self {
            editing_id: None,
            name: String::new(),
            username: "root".to_string(),
            auth_method: AuthMethod::Password,
            password: String::new(),
            ssh_key_id: None,
            private_key_path: String::new(),
            passphrase: String::new(),
            status: None,
            error: None,
        }
    }
}

// ---------------------------------------------------------------------------
// 2. Host Group Modal
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct GroupModalState {
    pub editing_id: Option<String>,
    pub name: String,
    pub parent_id: Option<String>,
    pub identity_id: Option<String>,
    pub proxy_jump_id: Option<String>,
    pub default_port: String,
    pub tags: String,
    pub error: Option<String>,
}

impl Default for GroupModalState {
    fn default() -> Self {
        Self {
            editing_id: None,
            name: String::new(),
            parent_id: None,
            identity_id: None,
            proxy_jump_id: None,
            default_port: "22".to_string(),
            tags: String::new(),
            error: None,
        }
    }
}

// ---------------------------------------------------------------------------
// 3. Host Importer Modal (~/.ssh/config + Export, PuTTY/CSV, Cloud)
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostImportTab {
    SshConfig,
    PuttyCsv,
    Cloud,
}

#[derive(Debug, Clone)]
pub struct HostImportModalState {
    pub tab: HostImportTab,
    pub ssh_config_path: String,
    pub putty_csv_path: String,
    pub putty_text: String,
    pub preview: Vec<ServerConfigInput>,
    pub selected: Vec<bool>,
    pub target_group_id: Option<String>,
    pub cloud_provider: CloudProvider,
    pub cloud_token: String,
    pub cloud_tailnet: String,
    pub loading: bool,
    pub status: Option<String>,
    pub error: Option<String>,
}

impl Default for HostImportModalState {
    fn default() -> Self {
        let default_ssh = default_ssh_config_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "~/.ssh/config".to_string());
        Self {
            tab: HostImportTab::SshConfig,
            ssh_config_path: default_ssh,
            putty_csv_path: String::new(),
            putty_text: String::new(),
            preview: Vec::new(),
            selected: Vec::new(),
            target_group_id: None,
            cloud_provider: CloudProvider::DigitalOcean,
            cloud_token: String::new(),
            cloud_tailnet: "-".to_string(),
            loading: false,
            status: None,
            error: None,
        }
    }
}

// ---------------------------------------------------------------------------
// 4. Snippet {{var}} Parameter Prompt Modal
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct SnippetVarsModalState {
    pub title: String,
    pub template: String,
    pub vars: Vec<(String, String)>,
    pub window_id: window::Id,
    pub snippet_id: Option<u64>,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// 5. Group Multi-Host Execution Modal
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct GroupRunTargetResult {
    pub server_id: String,
    pub server_name: String,
    pub status: String,
    pub output: String,
}

#[derive(Debug, Clone)]
pub struct GroupRunModalState {
    pub title: String,
    pub command: String,
    pub selected_group_id: Option<String>,
    pub results: Vec<GroupRunTargetResult>,
    pub running: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum TermiusModal {
    Identities(IdentitiesModalState),
    Group(GroupModalState),
    ImportHosts(HostImportModalState),
    SnippetVars(SnippetVarsModalState),
    GroupRun(GroupRunModalState),
}

#[derive(Debug, Clone)]
pub enum TermiusModalMsg {
    Close,
    Noop,

    // Identities
    IdentitySelectEdit(String),
    IdentityNew,
    IdentityDelete(String),
    IdentityName(String),
    IdentityUsername(String),
    IdentityAuthMethod(AuthMethod),
    IdentityPassword(String),
    IdentitySshKey(Option<String>),
    IdentityKeyPath(String),
    IdentityBrowseKey,
    IdentityPassphrase(String),
    IdentitySave,

    // Host Group
    GroupName(String),
    GroupParent(Option<String>),
    GroupIdentity(Option<String>),
    GroupJumpHost(Option<String>),
    GroupDefaultPort(String),
    GroupTags(String),
    GroupSave,
    GroupDelete(String),

    // Host Importer
    ImportTab(HostImportTab),
    ImportSshPath(String),
    ImportBrowseSshPath,
    ImportParseSshConfig,
    ImportExportSshConfig,
    ImportBrowsePuttyCsv,
    ImportPuttyText(String),
    ImportParsePuttyCsv,
    ImportCloudProvider(CloudProvider),
    ImportCloudToken(String),
    ImportCloudTailnet(String),
    ImportFetchCloud,
    ImportTargetGroup(Option<String>),
    ImportToggleRow(usize, bool),
    ImportSelectAll(bool),
    ImportCommit,

    // Snippet Variables
    SnippetVarChanged(usize, String),
    SnippetVarRun,

    // Group Run
    GroupRunCommandChanged(String),
    GroupRunGroupChanged(Option<String>),
    GroupRunExecute,
}

fn backdrop_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(0.02, 0.024, 0.045, 0.72))),
        snap: true,
        ..Default::default()
    }
}

fn active_pill_button(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        if active {
            button::Style {
                background: Some(Background::Color(rgba(BRAND_500, 0.25))),
                text_color: BRAND_400,
                border: Border {
                    color: BRAND_500,
                    width: 1.0,
                    radius: 8.0.into(),
                },
                snap: true,
                ..Default::default()
            }
        } else {
            secondary_button(theme, status)
        }
    }
}

fn modal_shell<'a>(
    title: &'a str,
    body: Element<'a, TermiusModalMsg>,
    width: f32,
) -> Element<'a, TermiusModalMsg> {
    let header = row![
        text(title)
            .size(17)
            .font(FONT_MONO_BOLD)
            .color(TEXT_PRIMARY)
            .width(Length::Fill),
        button(text("Close").size(12).color(TEXT_LIGHT))
            .on_press(TermiusModalMsg::Close)
            .padding([5, 11])
            .style(secondary_button),
    ]
    .align_y(alignment::Vertical::Center);

    let card = mouse_area(
        container(column![header, space().height(12), body].width(Length::Fill))
            .width(Length::Fixed(width))
            .padding(20)
            .style(modal_card),
    )
    .on_press(TermiusModalMsg::Noop);

    let backdrop = mouse_area(
        container(space().width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(backdrop_style),
    )
    .on_press(TermiusModalMsg::Close);

    let fg = container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .padding(24);

    stack![backdrop, fg]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

pub fn view<'a>(
    modal: &'a TermiusModal,
    identities: &'a [IdentitySummary],
    groups: &'a [HostGroup],
    servers: &'a [ServerSummary],
    ssh_keys: &'a [SshKeySummary],
) -> Element<'a, TermiusModalMsg> {
    match modal {
        TermiusModal::Identities(st) => view_identities(st, identities, ssh_keys),
        TermiusModal::Group(st) => view_group(st, groups, identities, servers),
        TermiusModal::ImportHosts(st) => view_import_hosts(st, groups),
        TermiusModal::SnippetVars(st) => view_snippet_vars(st),
        TermiusModal::GroupRun(st) => view_group_run(st, groups),
    }
}

fn view_identities<'a>(
    st: &'a IdentitiesModalState,
    identities: &'a [IdentitySummary],
    ssh_keys: &'a [SshKeySummary],
) -> Element<'a, TermiusModalMsg> {
    let mut left_col = column![
        row![
            text("Saved Identities")
                .size(13)
                .font(FONT_MONO_BOLD)
                .color(TEXT_LIGHT)
                .width(Length::Fill),
            button(text("+ New").size(11))
                .on_press(TermiusModalMsg::IdentityNew)
                .padding([4, 9])
                .style(active_pill_button(st.editing_id.is_none())),
        ]
        .align_y(alignment::Vertical::Center),
        space().height(6),
    ]
    .spacing(6);

    if identities.is_empty() {
        left_col = left_col.push(
            text("No reusable identities yet.")
                .size(12)
                .color(TEXT_MUTED),
        );
    } else {
        for ident in identities {
            let selected = st.editing_id.as_deref() == Some(ident.id.as_str());
            let row_btn = button(
                column![
                    text(truncate_ellipsis(&ident.name, 26))
                        .size(13)
                        .color(TEXT_PRIMARY),
                    text(format!("{} • {}", ident.username, ident.auth_method))
                        .size(11)
                        .color(TEXT_SECONDARY),
                ]
                .spacing(2),
            )
            .on_press(TermiusModalMsg::IdentitySelectEdit(ident.id.clone()))
            .padding([8, 10])
            .width(Length::Fill)
            .style(active_pill_button(selected));
            left_col = left_col.push(row_btn);
        }
    }

    let left_pane = container(
        scrollable(left_col)
            .height(Length::Fixed(340.0))
            .style(dark_scrollable),
    )
    .width(Length::Fixed(220.0))
    .padding(10)
    .style(glass_panel);

    let key_choices: Vec<OptionChoice> = std::iter::once(OptionChoice {
        id: String::new(),
        label: "(None selected)".to_string(),
    })
    .chain(ssh_keys.iter().map(|k| OptionChoice {
        id: k.id.clone(),
        label: format!("{} ({})", k.name, k.algorithm),
    }))
    .collect();
    let sel_key = key_choices
        .iter()
        .find(|c| {
            st.ssh_key_id
                .as_deref()
                .map(|id| id == c.id)
                .unwrap_or(c.id.is_empty())
        })
        .cloned();

    let mut form = column![
        text(if st.editing_id.is_some() {
            "Edit Identity"
        } else {
            "Create Identity"
        })
        .size(14)
        .font(FONT_MONO_BOLD)
        .color(TEXT_PRIMARY),
        row![
            text("Label")
                .size(12)
                .color(TEXT_SECONDARY)
                .width(Length::Fixed(100.0)),
            text_input("e.g. Prod Root Key", &st.name)
                .on_input(TermiusModalMsg::IdentityName)
                .padding([7, 10])
                .size(13)
                .style(dark_input),
        ]
        .align_y(alignment::Vertical::Center),
        row![
            text("Username")
                .size(12)
                .color(TEXT_SECONDARY)
                .width(Length::Fixed(100.0)),
            text_input("root", &st.username)
                .on_input(TermiusModalMsg::IdentityUsername)
                .padding([7, 10])
                .size(13)
                .style(dark_input),
        ]
        .align_y(alignment::Vertical::Center),
        row![
            text("Auth Type")
                .size(12)
                .color(TEXT_SECONDARY)
                .width(Length::Fixed(100.0)),
            pick_list(
                vec![
                    AuthMethod::Password,
                    AuthMethod::KeyFile,
                    AuthMethod::VaultKey,
                    AuthMethod::Agent,
                ],
                Some(st.auth_method),
                TermiusModalMsg::IdentityAuthMethod,
            )
            .text_size(12)
            .padding([6, 10])
            .style(dark_pick_list),
        ]
        .align_y(alignment::Vertical::Center),
    ]
    .spacing(10)
    .width(Length::Fill);

    match st.auth_method {
        AuthMethod::Password => {
            form = form.push(
                row![
                    text("Password")
                        .size(12)
                        .color(TEXT_SECONDARY)
                        .width(Length::Fixed(100.0)),
                    text_input(
                        if st.editing_id.is_some() {
                            "Leave blank to keep existing"
                        } else {
                            "Password"
                        },
                        &st.password
                    )
                    .secure(true)
                    .on_input(TermiusModalMsg::IdentityPassword)
                    .padding([7, 10])
                    .size(13)
                    .style(dark_input),
                ]
                .align_y(alignment::Vertical::Center),
            );
        }
        AuthMethod::VaultKey => {
            form = form.push(
                row![
                    text("SSH Key")
                        .size(12)
                        .color(TEXT_SECONDARY)
                        .width(Length::Fixed(100.0)),
                    pick_list(key_choices, sel_key, |c| {
                        TermiusModalMsg::IdentitySshKey((!c.id.is_empty()).then_some(c.id))
                    })
                    .text_size(12)
                    .padding([6, 10])
                    .style(dark_pick_list),
                ]
                .align_y(alignment::Vertical::Center),
            );
        }
        AuthMethod::KeyFile => {
            form = form
                .push(
                    row![
                        text("Key File")
                            .size(12)
                            .color(TEXT_SECONDARY)
                            .width(Length::Fixed(100.0)),
                        text_input("~/.ssh/id_ed25519", &st.private_key_path)
                            .on_input(TermiusModalMsg::IdentityKeyPath)
                            .padding([7, 10])
                            .size(13)
                            .style(dark_input),
                        button(text("Browse").size(12).color(TEXT_LIGHT))
                            .on_press(TermiusModalMsg::IdentityBrowseKey)
                            .padding([7, 10])
                            .style(secondary_button),
                    ]
                    .spacing(6)
                    .align_y(alignment::Vertical::Center),
                )
                .push(
                    row![
                        text("Passphrase")
                            .size(12)
                            .color(TEXT_SECONDARY)
                            .width(Length::Fixed(100.0)),
                        text_input("Optional key passphrase", &st.passphrase)
                            .secure(true)
                            .on_input(TermiusModalMsg::IdentityPassphrase)
                            .padding([7, 10])
                            .size(13)
                            .style(dark_input),
                    ]
                    .align_y(alignment::Vertical::Center),
                );
        }
        AuthMethod::Agent => {
            form = form.push(
                text("Authenticates using local OpenSSH Agent / Pageant.")
                    .size(12)
                    .color(TEXT_SECONDARY),
            );
        }
    }

    if let Some(err) = &st.error {
        form = form.push(text(err).size(12).color(RED_400));
    } else if let Some(msg) = &st.status {
        form = form.push(text(msg).size(12).color(GREEN_400));
    }

    let mut actions = row![
        button(text("Save Identity").size(13).color(Color::WHITE))
            .on_press(TermiusModalMsg::IdentitySave)
            .padding([8, 16])
            .style(brand_button),
    ]
    .spacing(8);

    if let Some(id) = &st.editing_id {
        actions = actions.push(
            button(text("Delete").size(12))
                .on_press(TermiusModalMsg::IdentityDelete(id.clone()))
                .padding([8, 12])
                .style(danger_ghost_button),
        );
    }
    form = form.push(space().height(4)).push(actions);

    let body = row![
        left_pane,
        container(form).padding(14).width(Length::Fill).style(glass_panel),
    ]
    .spacing(12);

    modal_shell("Reusable Credentials & Identities", body.into(), 720.0)
}

fn view_group<'a>(
    st: &'a GroupModalState,
    groups: &'a [HostGroup],
    identities: &'a [IdentitySummary],
    servers: &'a [ServerSummary],
) -> Element<'a, TermiusModalMsg> {
    let parent_choices: Vec<OptionChoice> = std::iter::once(OptionChoice {
        id: String::new(),
        label: "(Root Group)".to_string(),
    })
    .chain(
        groups
            .iter()
            .filter(|g| st.editing_id.as_deref() != Some(g.id.as_str()))
            .map(|g| OptionChoice {
                id: g.id.clone(),
                label: g.name.clone(),
            }),
    )
    .collect();
    let sel_parent = parent_choices
        .iter()
        .find(|c| {
            st.parent_id
                .as_deref()
                .map(|id| id == c.id)
                .unwrap_or(c.id.is_empty())
        })
        .cloned();

    let id_choices: Vec<OptionChoice> = std::iter::once(OptionChoice {
        id: String::new(),
        label: "(Inherit from parent)".to_string(),
    })
    .chain(identities.iter().map(|i| OptionChoice {
        id: i.id.clone(),
        label: format!("{} ({})", i.name, i.username),
    }))
    .collect();
    let sel_id = id_choices
        .iter()
        .find(|c| {
            st.identity_id
                .as_deref()
                .map(|id| id == c.id)
                .unwrap_or(c.id.is_empty())
        })
        .cloned();

    let jump_choices: Vec<OptionChoice> = std::iter::once(OptionChoice {
        id: String::new(),
        label: "(None)".to_string(),
    })
    .chain(servers.iter().map(|s| OptionChoice {
        id: s.id.clone(),
        label: format!("{} ({})", s.name, s.host),
    }))
    .collect();
    let sel_jump = jump_choices
        .iter()
        .find(|c| {
            st.proxy_jump_id
                .as_deref()
                .map(|id| id == c.id)
                .unwrap_or(c.id.is_empty())
        })
        .cloned();

    let mut col = column![
        row![
            text("Group Name")
                .size(12)
                .color(TEXT_SECONDARY)
                .width(Length::Fixed(130.0)),
            text_input("e.g. Production / US-East", &st.name)
                .on_input(TermiusModalMsg::GroupName)
                .padding([7, 10])
                .size(13)
                .style(dark_input),
        ]
        .align_y(alignment::Vertical::Center),
        row![
            text("Parent Group")
                .size(12)
                .color(TEXT_SECONDARY)
                .width(Length::Fixed(130.0)),
            pick_list(parent_choices, sel_parent, |c| {
                TermiusModalMsg::GroupParent((!c.id.is_empty()).then_some(c.id))
            })
            .text_size(12)
            .padding([6, 10])
            .style(dark_pick_list),
        ]
        .align_y(alignment::Vertical::Center),
        row![
            text("Default Identity")
                .size(12)
                .color(TEXT_SECONDARY)
                .width(Length::Fixed(130.0)),
            pick_list(id_choices, sel_id, |c| {
                TermiusModalMsg::GroupIdentity((!c.id.is_empty()).then_some(c.id))
            })
            .text_size(12)
            .padding([6, 10])
            .style(dark_pick_list),
        ]
        .align_y(alignment::Vertical::Center),
        row![
            text("Default Jump Host")
                .size(12)
                .color(TEXT_SECONDARY)
                .width(Length::Fixed(130.0)),
            pick_list(jump_choices, sel_jump, |c| {
                TermiusModalMsg::GroupJumpHost((!c.id.is_empty()).then_some(c.id))
            })
            .text_size(12)
            .padding([6, 10])
            .style(dark_pick_list),
        ]
        .align_y(alignment::Vertical::Center),
        row![
            text("Default Port")
                .size(12)
                .color(TEXT_SECONDARY)
                .width(Length::Fixed(130.0)),
            text_input("22", &st.default_port)
                .on_input(TermiusModalMsg::GroupDefaultPort)
                .padding([7, 10])
                .size(13)
                .width(Length::Fixed(90.0))
                .style(dark_input),
        ]
        .align_y(alignment::Vertical::Center),
        row![
            text("Group Tags")
                .size(12)
                .color(TEXT_SECONDARY)
                .width(Length::Fixed(130.0)),
            text_input("prod, aws, k8s (comma separated)", &st.tags)
                .on_input(TermiusModalMsg::GroupTags)
                .padding([7, 10])
                .size(13)
                .style(dark_input),
        ]
        .align_y(alignment::Vertical::Center),
    ]
    .spacing(10)
    .width(Length::Fill);

    if let Some(err) = &st.error {
        col = col.push(text(err).size(12).color(RED_400));
    }

    let mut btns = row![
        button(text("Save Group").size(13).color(Color::WHITE))
            .on_press(TermiusModalMsg::GroupSave)
            .padding([8, 16])
            .style(brand_button),
    ]
    .spacing(8);

    if let Some(gid) = &st.editing_id {
        btns = btns.push(
            button(text("Delete Group").size(12))
                .on_press(TermiusModalMsg::GroupDelete(gid.clone()))
                .padding([8, 12])
                .style(danger_ghost_button),
        );
    }

    col = col.push(space().height(6)).push(btns);
    let title = if st.editing_id.is_some() {
        "Edit Host Group"
    } else {
        "New Host Group"
    };
    modal_shell(title, col.into(), 520.0)
}

fn view_import_hosts<'a>(
    st: &'a HostImportModalState,
    groups: &'a [HostGroup],
) -> Element<'a, TermiusModalMsg> {
    let tabs = row![
        button(text("OpenSSH ~/.ssh/config").size(12))
            .on_press(TermiusModalMsg::ImportTab(HostImportTab::SshConfig))
            .padding([6, 12])
            .style(active_pill_button(st.tab == HostImportTab::SshConfig)),
        button(text("PuTTY .reg / CSV").size(12))
            .on_press(TermiusModalMsg::ImportTab(HostImportTab::PuttyCsv))
            .padding([6, 12])
            .style(active_pill_button(st.tab == HostImportTab::PuttyCsv)),
        button(
            row![
                icons::cloud_download(13.0, BRAND_400),
                space().width(5),
                text("Cloud Sync Import").size(12),
            ]
            .align_y(alignment::Vertical::Center),
        )
        .on_press(TermiusModalMsg::ImportTab(HostImportTab::Cloud))
        .padding([6, 12])
        .style(active_pill_button(st.tab == HostImportTab::Cloud)),
    ]
    .spacing(8);

    let tab_body: Element<'a, TermiusModalMsg> = match st.tab {
        HostImportTab::SshConfig => column![
            row![
                text_input("Path to ~/.ssh/config", &st.ssh_config_path)
                    .on_input(TermiusModalMsg::ImportSshPath)
                    .padding([7, 10])
                    .size(12)
                    .style(dark_input),
                button(text("Browse").size(12).color(TEXT_LIGHT))
                    .on_press(TermiusModalMsg::ImportBrowseSshPath)
                    .padding([7, 10])
                    .style(secondary_button),
                button(text("Parse Hosts").size(12).color(Color::WHITE))
                    .on_press(TermiusModalMsg::ImportParseSshConfig)
                    .padding([7, 12])
                    .style(brand_button),
                button(text("Export Vault -> ssh_config").size(12).color(TEXT_LIGHT))
                    .on_press(TermiusModalMsg::ImportExportSshConfig)
                    .padding([7, 12])
                    .style(secondary_button),
            ]
            .spacing(6)
            .align_y(alignment::Vertical::Center),
        ]
        .spacing(8)
        .into(),
        HostImportTab::PuttyCsv => column![
            row![
                button(text("Load .reg or .csv File...").size(12).color(TEXT_LIGHT))
                    .on_press(TermiusModalMsg::ImportBrowsePuttyCsv)
                    .padding([7, 12])
                    .style(secondary_button),
                text(if st.putty_csv_path.is_empty() {
                    "Or paste CSV rows: name,host,user,port,tags"
                } else {
                    st.putty_csv_path.as_str()
                })
                .size(12)
                .color(TEXT_SECONDARY),
            ]
            .spacing(8)
            .align_y(alignment::Vertical::Center),
            row![
                text_input("Paste CSV / PuTTY text...", &st.putty_text)
                    .on_input(TermiusModalMsg::ImportPuttyText)
                    .padding([7, 10])
                    .size(12)
                    .style(dark_input),
                button(text("Parse").size(12).color(Color::WHITE))
                    .on_press(TermiusModalMsg::ImportParsePuttyCsv)
                    .padding([7, 12])
                    .style(brand_button),
            ]
            .spacing(6)
            .align_y(alignment::Vertical::Center),
        ]
        .spacing(8)
        .into(),
        HostImportTab::Cloud => {
            let mut cloud_col = column![
                row![
                    text("Provider:")
                        .size(12)
                        .color(TEXT_SECONDARY)
                        .width(Length::Fixed(72.0)),
                    pick_list(
                        CloudProvider::ALL.to_vec(),
                        Some(st.cloud_provider),
                        TermiusModalMsg::ImportCloudProvider
                    )
                    .text_size(12)
                    .padding([5, 10])
                    .style(dark_pick_list),
                    text_input("API Token (stored only in memory)", &st.cloud_token)
                        .secure(true)
                        .on_input(TermiusModalMsg::ImportCloudToken)
                        .padding([7, 10])
                        .size(12)
                        .style(dark_input),
                    button(
                        text(if st.loading {
                            "Fetching..."
                        } else {
                            "Fetch Hosts"
                        })
                        .size(12)
                        .color(Color::WHITE),
                    )
                    .on_press_maybe((!st.loading).then_some(TermiusModalMsg::ImportFetchCloud))
                    .padding([7, 12])
                    .style(brand_button),
                ]
                .spacing(8)
                .align_y(alignment::Vertical::Center),
            ]
            .spacing(8);

            if st.cloud_provider == CloudProvider::Tailscale {
                cloud_col = cloud_col.push(
                    row![
                        text("Tailnet:")
                            .size(12)
                            .color(TEXT_SECONDARY)
                            .width(Length::Fixed(72.0)),
                        text_input("- (default) or example.com", &st.cloud_tailnet)
                            .on_input(TermiusModalMsg::ImportCloudTailnet)
                            .padding([6, 10])
                            .size(12)
                            .width(Length::Fixed(240.0))
                            .style(dark_input),
                    ]
                    .spacing(8)
                    .align_y(alignment::Vertical::Center),
                );
            }
            cloud_col.into()
        }
    };

    let group_choices: Vec<OptionChoice> = std::iter::once(OptionChoice {
        id: String::new(),
        label: "(No Group / Root)".to_string(),
    })
    .chain(groups.iter().map(|g| OptionChoice {
        id: g.id.clone(),
        label: g.name.clone(),
    }))
    .collect();
    let sel_group = group_choices
        .iter()
        .find(|c| {
            st.target_group_id
                .as_deref()
                .map(|id| id == c.id)
                .unwrap_or(c.id.is_empty())
        })
        .cloned();

    let mut preview_col = column![].spacing(4);
    for (idx, host) in st.preview.iter().enumerate() {
        let checked = st.selected.get(idx).copied().unwrap_or(true);
        preview_col = preview_col.push(
            container(
                row![
                    checkbox(checked).on_toggle(move |v| TermiusModalMsg::ImportToggleRow(idx, v)),
                    text(truncate_ellipsis(&host.name, 26))
                        .size(12)
                        .color(TEXT_PRIMARY)
                        .width(Length::Fixed(170.0)),
                    text(format!("{}@{}:{}", host.username, host.host, host.port))
                        .size(12)
                        .color(TEXT_LIGHT)
                        .width(Length::Fill),
                ]
                .spacing(8)
                .align_y(alignment::Vertical::Center),
            )
            .padding([6, 8])
            .style(glass_panel),
        );
    }

    let selected_count = st.selected.iter().filter(|&&b| b).count();
    let mut main = column![
        tabs,
        container(tab_body)
            .padding(12)
            .width(Length::Fill)
            .style(glass_panel),
        row![
            text(format!(
                "{} host(s) parsed ({} selected)",
                st.preview.len(),
                selected_count
            ))
            .size(12)
            .color(TEXT_LIGHT)
            .width(Length::Fill),
            button(text("Select All").size(11).color(TEXT_LIGHT))
                .on_press(TermiusModalMsg::ImportSelectAll(true))
                .padding([4, 8])
                .style(secondary_button),
            button(text("None").size(11).color(TEXT_LIGHT))
                .on_press(TermiusModalMsg::ImportSelectAll(false))
                .padding([4, 8])
                .style(secondary_button),
        ]
        .spacing(6)
        .align_y(alignment::Vertical::Center),
        scrollable(preview_col)
            .height(Length::Fixed(210.0))
            .style(dark_scrollable),
        row![
            text("Target Group:").size(12).color(TEXT_SECONDARY),
            pick_list(group_choices, sel_group, |c| {
                TermiusModalMsg::ImportTargetGroup((!c.id.is_empty()).then_some(c.id))
            })
            .text_size(12)
            .padding([5, 10])
            .style(dark_pick_list),
            space().width(Length::Fill),
            button(
                text(format!("Import {selected_count} Host(s)"))
                    .size(13)
                    .color(Color::WHITE),
            )
            .on_press_maybe((selected_count > 0).then_some(TermiusModalMsg::ImportCommit))
            .padding([8, 16])
            .style(brand_button),
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center),
    ]
    .spacing(10);

    if let Some(err) = &st.error {
        main = main.push(text(err).size(12).color(RED_400));
    } else if let Some(msg) = &st.status {
        main = main.push(text(msg).size(12).color(GREEN_400));
    }

    modal_shell("Import & Export Hosts", main.into(), 720.0)
}

fn view_snippet_vars<'a>(st: &'a SnippetVarsModalState) -> Element<'a, TermiusModalMsg> {
    let mut rendered = st.template.clone();
    for (k, v) in &st.vars {
        rendered = rendered.replace(&format!("{{{{{k}}}}}"), v);
    }

    let mut col = column![text(format!("Snippet: {}", st.title))
        .size(13)
        .color(TEXT_LIGHT)]
    .spacing(10);

    for (idx, (key, val)) in st.vars.iter().enumerate() {
        col = col.push(
            row![
                text(format!("{{{{{key}}}}}"))
                    .size(12)
                    .color(BRAND_400)
                    .width(Length::Fixed(140.0)),
                text_input("Enter value...", val)
                    .on_input(move |v| TermiusModalMsg::SnippetVarChanged(idx, v))
                    .on_submit(TermiusModalMsg::SnippetVarRun)
                    .padding([7, 10])
                    .size(13)
                    .style(dark_input),
            ]
            .spacing(8)
            .align_y(alignment::Vertical::Center),
        );
    }

    col = col
        .push(
            container(
                column![
                    text("Rendered Command Preview").size(11).color(TEXT_SECONDARY),
                    text(rendered).size(12).color(GREEN_400),
                ]
                .spacing(4),
            )
            .padding(10)
            .width(Length::Fill)
            .style(glass_panel),
        )
        .push(row![
            space().width(Length::Fill),
            button(text("Execute in Terminal").size(13).color(Color::WHITE))
                .on_press(TermiusModalMsg::SnippetVarRun)
                .padding([8, 16])
                .style(brand_button),
        ]);

    if let Some(err) = &st.error {
        col = col.push(text(err).size(12).color(RED_400));
    }

    modal_shell("Parameterized Snippet", col.into(), 520.0)
}

fn view_group_run<'a>(
    st: &'a GroupRunModalState,
    groups: &'a [HostGroup],
) -> Element<'a, TermiusModalMsg> {
    let group_choices: Vec<OptionChoice> = std::iter::once(OptionChoice {
        id: String::new(),
        label: "(All Saved Hosts)".to_string(),
    })
    .chain(groups.iter().map(|g| OptionChoice {
        id: g.id.clone(),
        label: g.name.clone(),
    }))
    .collect();
    let sel = group_choices
        .iter()
        .find(|c| {
            st.selected_group_id
                .as_deref()
                .map(|id| id == c.id)
                .unwrap_or(c.id.is_empty())
        })
        .cloned();

    let mut res_col = column![].spacing(6);
    if st.results.is_empty() {
        res_col = res_col.push(
            text("Choose a target group and run a command over SSH in parallel.")
                .size(12)
                .color(TEXT_SECONDARY),
        );
    } else {
        for r in &st.results {
            let status_color = match r.status.as_str() {
                "Success" => GREEN_400,
                "Failed" => RED_400,
                _ => BRAND_400,
            };
            res_col = res_col.push(
                container(
                    column![
                        row![
                            text(&r.server_name)
                                .size(13)
                                .font(FONT_MONO_BOLD)
                                .color(TEXT_PRIMARY)
                                .width(Length::Fill),
                            text(&r.status).size(11).color(status_color),
                        ],
                        text(truncate_ellipsis(&r.output, 300))
                            .size(11)
                            .color(TEXT_LIGHT),
                    ]
                    .spacing(4),
                )
                .padding(10)
                .width(Length::Fill)
                .style(glass_panel),
            );
        }
    }

    let mut col = column![
        text(if st.title.trim().is_empty() {
            "Parallel SSH Execution".to_string()
        } else {
            format!("Snippet: {}", st.title)
        })
        .size(13)
        .color(TEXT_LIGHT),
        row![
            text("Target Group:").size(12).color(TEXT_SECONDARY),
            pick_list(group_choices, sel, |c| {
                TermiusModalMsg::GroupRunGroupChanged((!c.id.is_empty()).then_some(c.id))
            })
            .text_size(12)
            .padding([5, 10])
            .style(dark_pick_list),
            space().width(Length::Fill),
            button(
                text(if st.running {
                    "Running..."
                } else {
                    "Execute on Group"
                })
                .size(12)
                .color(Color::WHITE),
            )
            .on_press_maybe((!st.running).then_some(TermiusModalMsg::GroupRunExecute))
            .padding([7, 14])
            .style(brand_button),
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center),
        text_input("Shell command to execute...", &st.command)
            .on_input(TermiusModalMsg::GroupRunCommandChanged)
            .padding([8, 10])
            .size(13)
            .style(dark_input),
        scrollable(res_col)
            .height(Length::Fixed(270.0))
            .style(dark_scrollable),
    ]
    .spacing(10);

    if let Some(err) = &st.error {
        col = col.push(text(err).size(12).color(RED_400));
    }

    modal_shell("Run Command Across Host Group", col.into(), 660.0)
}
