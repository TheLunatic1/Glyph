use crate::theme::{
    brand_button, danger_button, dark_card, dark_input, dark_pick_list, dark_scrollable,
    ghost_icon_button, glass_panel, pill_badge, rgba, secondary_button, truncate_ellipsis,
    BRAND_300, BRAND_400, BRAND_500, DARK_700, DARK_800, DARK_900, FONT_MONO, FONT_MONO_BOLD,
    FONT_UI_BOLD, FONT_UI_SEMIBOLD, GREEN_400, GREEN_500, RED_400, RED_500, TEXT_LIGHT,
    TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY, YELLOW_400, YELLOW_500,
};
use crate::ui::icons;
use glyph_core::containers::{ContainerAction, ContainerDetails, ContainerSummary};
use iced::widget::{
    button, column, container, pick_list, row, scrollable, space, text, text_input,
};
use iced::{alignment, Background, Border, Color, Element, Length};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerSubTab {
    Logs,
    Inspect,
    Settings,
}

#[derive(Debug, Clone)]
pub struct ContainerTabDetail {
    pub summary: ContainerSummary,
    pub subtab: ContainerSubTab,
    pub logs: String,
    pub inspect: Option<ContainerDetails>,
    pub loading: bool,
    pub restart_policy: String,
    pub mem_limit: String,
    pub cpu_limit: String,
    pub update_status: Option<String>,
}

impl ContainerTabDetail {
    pub fn new(summary: ContainerSummary) -> Self {
        Self {
            summary,
            subtab: ContainerSubTab::Logs,
            logs: String::new(),
            inspect: None,
            loading: true,
            restart_policy: "unless-stopped".to_string(),
            mem_limit: String::new(),
            cpu_limit: String::new(),
            update_status: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContainersPageState {
    pub loaded_once: bool,
    pub loading: bool,
    pub search: String,
    pub containers: Vec<ContainerSummary>,
    pub show_compose: bool,
    pub compose_path: String,
    pub compose_busy: bool,
    pub compose_output: Option<String>,
    pub open_tabs: Vec<String>,
    pub active_tab: Option<String>, // None = Container List
    pub details: HashMap<String, ContainerTabDetail>,
    pub error: Option<String>,
}

impl Default for ContainersPageState {
    fn default() -> Self {
        Self {
            loaded_once: false,
            loading: false,
            search: String::new(),
            containers: Vec::new(),
            show_compose: false,
            compose_path: "/srv".to_string(),
            compose_busy: false,
            compose_output: None,
            open_tabs: Vec::new(),
            active_tab: None,
            details: HashMap::new(),
            error: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ContainersPageMsg {
    SelectListTab,
    OpenContainerTab(ContainerSummary),
    SwitchContainerTab(String),
    CloseContainerTab(String),
    Refresh,
    SearchChanged(String),
    ToggleCompose,
    ComposePathChanged(String),
    ComposeUp,
    ComposeDown,
    Action(String, ContainerAction),
    SelectSubTab(String, ContainerSubTab),
    RefreshDetail(String),
    RestartPolicyChanged(String, String),
    MemLimitChanged(String, String),
    CpuLimitChanged(String, String),
    ApplyUpdate(String),
}

pub fn view<'a>(st: &'a ContainersPageState) -> Element<'a, ContainersPageMsg> {
    let tab_bar = build_tab_bar(st);

    let main_body: Element<'a, ContainersPageMsg> = if let Some(cid) = &st.active_tab {
        if let Some(detail) = st.details.get(cid) {
            view_detail(detail, st.error.as_deref())
        } else {
            view_list(st)
        }
    } else {
        view_list(st)
    };

    column![tab_bar, main_body]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn build_tab_bar<'a>(st: &'a ContainersPageState) -> Element<'a, ContainersPageMsg> {
    let is_list = st.active_tab.is_none();
    let mut tabs = row![button(
        row![
            icons::cube(14.0, if is_list { BRAND_400 } else { TEXT_SECONDARY }),
            text("Container List")
                .size(12)
                .font(FONT_UI_SEMIBOLD)
                .color(if is_list { TEXT_PRIMARY } else { TEXT_SECONDARY }),
        ]
        .spacing(6)
        .align_y(alignment::Vertical::Center),
    )
    .padding([8, 14])
    .style(tab_btn_style(is_list))
    .on_press(ContainersPageMsg::SelectListTab)]
    .spacing(6)
    .align_y(alignment::Vertical::Center);

    for cid in &st.open_tabs {
        if let Some(det) = st.details.get(cid) {
            let active = st.active_tab.as_deref() == Some(cid.as_str());
            let cid_click = cid.clone();
            let cid_close = cid.clone();
            let tab_item = row![
                button(
                    text(&det.summary.name)
                        .size(12)
                        .font(FONT_MONO)
                        .color(if active { TEXT_PRIMARY } else { TEXT_SECONDARY }),
                )
                .padding([8, 10])
                .style(tab_btn_style(active))
                .on_press(ContainersPageMsg::SwitchContainerTab(cid_click)),
                button(icons::x_close(12.0, TEXT_MUTED))
                    .padding([6, 6])
                    .style(ghost_icon_button)
                    .on_press(ContainersPageMsg::CloseContainerTab(cid_close)),
            ]
            .spacing(2)
            .align_y(alignment::Vertical::Center);
            tabs = tabs.push(tab_item);
        }
    }

    container(
        scrollable(tabs)
            .horizontal()
            .width(Length::Fill)
            .style(dark_scrollable),
    )
    .width(Length::Fill)
    .padding([8, 20])
    .style(|_| container::Style {
        background: Some(Background::Color(DARK_800)),
        border: Border {
            color: DARK_700,
            width: 1.0,
            radius: 0.0.into(),
        },
        snap: true,
        ..Default::default()
    })
    .into()
}

fn tab_btn_style(active: bool) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, status| {
        let bg = if active {
            DARK_900
        } else if matches!(status, button::Status::Hovered) {
            DARK_700
        } else {
            Color::TRANSPARENT
        };
        button::Style {
            background: Some(Background::Color(bg)),
            border: Border {
                color: if active { BRAND_500 } else { Color::TRANSPARENT },
                width: if active { 1.0 } else { 0.0 },
                radius: 8.0.into(),
            },
            snap: true,
            ..Default::default()
        }
    }
}

fn view_list<'a>(st: &'a ContainersPageState) -> Element<'a, ContainersPageMsg> {
    let header = row![
        container(icons::cube(20.0, BRAND_400))
            .padding(10)
            .style(|_| container::Style {
                background: Some(Background::Color(rgba(BRAND_500, 0.14))),
                border: Border {
                    color: rgba(BRAND_500, 0.3),
                    width: 1.0,
                    radius: 10.0.into(),
                },
                snap: true,
                ..Default::default()
            }),
        column![
            text("Docker Containers")
                .size(20)
                .font(FONT_UI_BOLD)
                .color(TEXT_PRIMARY),
            text("Manage Docker & Podman containers, logs, and Compose stacks")
                .size(12)
                .color(TEXT_SECONDARY),
        ]
        .spacing(2),
        space().width(Length::Fill),
        button(
            row![
                icons::layers(14.0, TEXT_LIGHT),
                text("Docker Compose").size(12).color(TEXT_LIGHT),
            ]
            .spacing(6)
            .align_y(alignment::Vertical::Center),
        )
        .padding([8, 12])
        .style(secondary_button)
        .on_press(ContainersPageMsg::ToggleCompose),
        button(
            row![
                icons::refresh_cw(14.0, TEXT_LIGHT),
                text(if st.loading { "Refreshing..." } else { "Refresh" })
                    .size(12)
                    .color(TEXT_LIGHT),
            ]
            .spacing(6)
            .align_y(alignment::Vertical::Center),
        )
        .padding([8, 12])
        .style(secondary_button)
        .on_press(ContainersPageMsg::Refresh),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center);

    let mut col = column![header].spacing(14).padding(22);

    if st.show_compose {
        let up_btn = button(
            text(if st.compose_busy { "Running..." } else { "Compose Up" })
                .size(12)
                .font(FONT_MONO_BOLD)
                .color(Color::WHITE),
        )
        .padding([8, 14])
        .style(brand_button);
        let down_btn = button(
            text("Compose Down")
                .size(12)
                .font(FONT_MONO_BOLD)
                .color(Color::WHITE),
        )
        .padding([8, 14])
        .style(danger_button);

        let mut compose_col = column![
            text("Docker Compose Project Directory")
                .size(12)
                .font(FONT_MONO_BOLD)
                .color(TEXT_LIGHT),
            row![
                text_input("/srv/my-stack", &st.compose_path)
                    .on_input(ContainersPageMsg::ComposePathChanged)
                    .padding([8, 12])
                    .size(13)
                    .font(FONT_MONO)
                    .width(Length::Fill)
                    .style(dark_input),
                if st.compose_busy {
                    up_btn
                } else {
                    up_btn.on_press(ContainersPageMsg::ComposeUp)
                },
                if st.compose_busy {
                    down_btn
                } else {
                    down_btn.on_press(ContainersPageMsg::ComposeDown)
                },
            ]
            .spacing(8)
            .align_y(alignment::Vertical::Center),
        ]
        .spacing(10);

        if let Some(out) = &st.compose_output {
            compose_col = compose_col.push(
                container(
                    scrollable(text(out).size(11).font(FONT_MONO).color(TEXT_LIGHT))
                        .height(Length::Fixed(110.0))
                        .style(dark_scrollable),
                )
                .width(Length::Fill)
                .padding(10)
                .style(dark_card),
            );
        }

        col = col.push(
            container(compose_col)
                .width(Length::Fill)
                .padding(14)
                .style(glass_panel),
        );
    }

    if let Some(err) = &st.error {
        col = col.push(
            container(text(err).size(12).color(RED_400))
                .width(Length::Fill)
                .padding([8, 12])
                .style(dark_card),
        );
    }

    let q = st.search.trim().to_ascii_lowercase();
    let filtered: Vec<&ContainerSummary> = st
        .containers
        .iter()
        .filter(|c| {
            q.is_empty()
                || c.name.to_ascii_lowercase().contains(&q)
                || c.image.to_ascii_lowercase().contains(&q)
                || c.id.to_ascii_lowercase().contains(&q)
        })
        .collect();

    let search_bar = row![
        text_input("Filter containers by name, image, or ID...", &st.search)
            .on_input(ContainersPageMsg::SearchChanged)
            .padding([8, 12])
            .size(13)
            .width(Length::Fill)
            .style(dark_input),
        text(format!("{} of {} containers", filtered.len(), st.containers.len()))
            .size(12)
            .font(FONT_MONO)
            .color(TEXT_MUTED),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center);

    col = col.push(search_bar);

    let head = row![
        text("Name")
            .size(11)
            .font(FONT_MONO_BOLD)
            .color(TEXT_MUTED)
            .width(Length::FillPortion(3)),
        text("Image")
            .size(11)
            .font(FONT_MONO_BOLD)
            .color(TEXT_MUTED)
            .width(Length::FillPortion(3)),
        text("State")
            .size(11)
            .font(FONT_MONO_BOLD)
            .color(TEXT_MUTED)
            .width(Length::Fixed(100.0)),
        text("Status")
            .size(11)
            .font(FONT_MONO_BOLD)
            .color(TEXT_MUTED)
            .width(Length::FillPortion(2)),
        text("Actions")
            .size(11)
            .font(FONT_MONO_BOLD)
            .color(TEXT_MUTED)
            .width(Length::Fixed(140.0)),
    ]
    .padding([8, 12]);

    let mut rows = column![head].spacing(6);
    if st.loading && st.containers.is_empty() {
        rows = rows.push(
            container(text("Querying container runtime...").size(12).color(TEXT_MUTED))
                .width(Length::Fill)
                .padding(28)
                .align_x(alignment::Horizontal::Center),
        );
    } else if filtered.is_empty() {
        rows = rows.push(
            container(
                column![
                    icons::cube(34.0, TEXT_MUTED),
                    text("No containers found").size(13).color(TEXT_SECONDARY),
                ]
                .spacing(6)
                .align_x(alignment::Horizontal::Center),
            )
            .width(Length::Fill)
            .padding(28)
            .align_x(alignment::Horizontal::Center),
        );
    } else {
        for c in filtered {
            let is_running = c.state.eq_ignore_ascii_case("running");
            let state_col = if is_running {
                GREEN_400
            } else if c.state.eq_ignore_ascii_case("exited") {
                RED_400
            } else {
                YELLOW_400
            };
            let state_bg = if is_running {
                rgba(GREEN_500, 0.14)
            } else if c.state.eq_ignore_ascii_case("exited") {
                rgba(RED_500, 0.14)
            } else {
                rgba(YELLOW_500, 0.14)
            };

            let cid = c.id.clone();
            let c_open = c.clone();

            let mut act_row = row![].spacing(4).width(Length::Fixed(140.0));
            if !is_running {
                act_row = act_row.push(
                    button(icons::play(13.0, GREEN_400))
                        .padding(6)
                        .style(ghost_icon_button)
                        .on_press(ContainersPageMsg::Action(cid.clone(), ContainerAction::Start)),
                );
            } else {
                act_row = act_row.push(
                    button(icons::square(12.0, RED_400))
                        .padding(6)
                        .style(ghost_icon_button)
                        .on_press(ContainersPageMsg::Action(cid.clone(), ContainerAction::Stop)),
                );
            }
            act_row = act_row.push(
                button(icons::rotate_cw(13.0, BRAND_300))
                    .padding(6)
                    .style(ghost_icon_button)
                    .on_press(ContainersPageMsg::Action(cid, ContainerAction::Restart)),
            );

            rows = rows.push(
                container(
                    row![
                        button(
                            text(truncate_ellipsis(&c.name, 26))
                                .size(13)
                                .font(FONT_MONO_BOLD)
                                .color(BRAND_300)
                                .wrapping(text::Wrapping::None),
                        )
                        .padding([4, 6])
                        .width(Length::FillPortion(3))
                        .clip(true)
                        .style(|_, _| button::Style {
                            background: None,
                            snap: true,
                            ..Default::default()
                        })
                        .on_press(ContainersPageMsg::OpenContainerTab(c_open)),
                        container(
                            text(truncate_ellipsis(&c.image, 28))
                                .size(12)
                                .font(FONT_MONO)
                                .color(TEXT_LIGHT)
                                .wrapping(text::Wrapping::None),
                        )
                        .width(Length::FillPortion(3))
                        .clip(true),
                        container(
                            container(
                                text(&c.state)
                                    .size(11)
                                    .font(FONT_MONO_BOLD)
                                    .color(state_col),
                            )
                            .padding([3, 8])
                            .style(pill_badge(state_bg, rgba(state_col, 0.35), state_col)),
                        )
                        .width(Length::Fixed(100.0)),
                        container(
                            text(truncate_ellipsis(&c.status, 24))
                                .size(12)
                                .color(TEXT_SECONDARY)
                                .wrapping(text::Wrapping::None),
                        )
                        .width(Length::FillPortion(2))
                        .clip(true),
                        act_row,
                    ]
                    .align_y(alignment::Vertical::Center),
                )
                .width(Length::Fill)
                .padding([8, 12])
                .style(dark_card),
            );
        }
    }

    col = col.push(
        container(
            scrollable(rows)
                .height(Length::Fill)
                .style(dark_scrollable),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(12)
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
    );

    col.width(Length::Fill).height(Length::Fill).into()
}

fn view_detail<'a>(
    det: &'a ContainerTabDetail,
    global_err: Option<&'a str>,
) -> Element<'a, ContainersPageMsg> {
    let cid = det.summary.id.clone();
    let is_running = det.summary.state.eq_ignore_ascii_case("running");

    let header = container(
        row![
            icons::cube(20.0, BRAND_400),
            column![
                text(&det.summary.name)
                    .size(18)
                    .font(FONT_UI_BOLD)
                    .color(TEXT_PRIMARY),
                text(format!("{}  ·  {}", det.summary.image, det.summary.status))
                    .size(12)
                    .font(FONT_MONO)
                    .color(TEXT_SECONDARY),
            ]
            .spacing(2),
            space().width(Length::Fill),
            if is_running {
                button(
                    row![
                        icons::square(12.0, Color::WHITE),
                        text("Stop").size(12).font(FONT_UI_SEMIBOLD).color(Color::WHITE),
                    ]
                    .spacing(6)
                    .align_y(alignment::Vertical::Center),
                )
                .padding([7, 12])
                .style(danger_button)
                .on_press(ContainersPageMsg::Action(cid.clone(), ContainerAction::Stop))
            } else {
                button(
                    row![
                        icons::play(12.0, Color::WHITE),
                        text("Start").size(12).font(FONT_UI_SEMIBOLD).color(Color::WHITE),
                    ]
                    .spacing(6)
                    .align_y(alignment::Vertical::Center),
                )
                .padding([7, 12])
                .style(brand_button)
                .on_press(ContainersPageMsg::Action(cid.clone(), ContainerAction::Start))
            },
            button(
                row![
                    icons::rotate_cw(13.0, TEXT_LIGHT),
                    text("Restart").size(12).color(TEXT_LIGHT),
                ]
                .spacing(6)
                .align_y(alignment::Vertical::Center),
            )
            .padding([7, 12])
            .style(secondary_button)
            .on_press(ContainersPageMsg::Action(cid.clone(), ContainerAction::Restart)),
            button(icons::refresh_cw(14.0, TEXT_LIGHT))
                .padding([7, 10])
                .style(secondary_button)
                .on_press(ContainersPageMsg::RefreshDetail(cid.clone())),
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center),
    )
    .width(Length::Fill)
    .padding(14)
    .style(glass_panel);

    let subtabs = row![
        subtab_btn("Logs", ContainerSubTab::Logs, det.subtab, cid.clone()),
        subtab_btn("Inspect", ContainerSubTab::Inspect, det.subtab, cid.clone()),
        subtab_btn("Settings", ContainerSubTab::Settings, det.subtab, cid.clone()),
    ]
    .spacing(8);

    let mut col = column![header, subtabs].spacing(14).padding(22);

    if let Some(err) = global_err {
        col = col.push(
            container(text(err).size(12).color(RED_400))
                .width(Length::Fill)
                .padding([8, 12])
                .style(dark_card),
        );
    }

    let body: Element<'a, ContainersPageMsg> = match det.subtab {
        ContainerSubTab::Logs => container(
            scrollable(
                text(if det.logs.is_empty() {
                    if det.loading {
                        "Fetching container logs..."
                    } else {
                        "(no log output)"
                    }
                } else {
                    det.logs.as_str()
                })
                .size(12)
                .font(FONT_MONO)
                .color(TEXT_LIGHT),
            )
            .height(Length::Fill)
            .style(dark_scrollable),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(14)
        .style(dark_card)
        .into(),

        ContainerSubTab::Inspect => {
            if let Some(ins) = &det.inspect {
                let stats = row![
                    mini_stat("Network", &ins.network_mode),
                    mini_stat("Ports", &ins.ports),
                    mini_stat("Restart Policy", &ins.restart_policy),
                    mini_stat("Memory", &ins.mem_usage),
                ]
                .spacing(10)
                .width(Length::Fill);

                let json_box = container(
                    scrollable(text(&ins.raw_json).size(11).font(FONT_MONO).color(TEXT_LIGHT))
                        .height(Length::Fill)
                        .style(dark_scrollable),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(14)
                .style(dark_card);

                column![stats, json_box]
                    .spacing(12)
                    .height(Length::Fill)
                    .into()
            } else {
                container(text("Loading container inspection data...").size(12).color(TEXT_MUTED))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(24)
                    .style(dark_card)
                    .into()
            }
        }

        ContainerSubTab::Settings => {
            let policies = vec![
                "no".to_string(),
                "always".to_string(),
                "unless-stopped".to_string(),
                "on-failure".to_string(),
            ];
            let cid_r = cid.clone();
            let cid_m = cid.clone();
            let cid_c = cid.clone();

            let mut form = column![
                text("Update Container Resources & Restart Policy")
                    .size(14)
                    .font(FONT_MONO_BOLD)
                    .color(TEXT_PRIMARY),
                row![
                    column![
                        text("Restart Policy").size(11).color(TEXT_SECONDARY),
                        pick_list(policies, Some(det.restart_policy.clone()), move |p| {
                            ContainersPageMsg::RestartPolicyChanged(cid_r.clone(), p)
                        })
                        .padding([8, 12])
                        .text_size(12)
                        .width(Length::Fill)
                        .style(dark_pick_list),
                    ]
                    .spacing(6)
                    .width(Length::FillPortion(1)),
                    column![
                        text("Memory Limit (e.g. 512m)").size(11).color(TEXT_SECONDARY),
                        text_input("512m", &det.mem_limit)
                            .on_input(move |v| ContainersPageMsg::MemLimitChanged(cid_m.clone(), v))
                            .padding([8, 12])
                            .size(12)
                            .font(FONT_MONO)
                            .style(dark_input),
                    ]
                    .spacing(6)
                    .width(Length::FillPortion(1)),
                    column![
                        text("CPUs (e.g. 1.5)").size(11).color(TEXT_SECONDARY),
                        text_input("1.5", &det.cpu_limit)
                            .on_input(move |v| ContainersPageMsg::CpuLimitChanged(cid_c.clone(), v))
                            .padding([8, 12])
                            .size(12)
                            .font(FONT_MONO)
                            .style(dark_input),
                    ]
                    .spacing(6)
                    .width(Length::FillPortion(1)),
                ]
                .spacing(12),
                row![
                    space().width(Length::Fill),
                    button(
                        text("Apply Docker Update")
                            .size(12)
                            .font(FONT_MONO_BOLD)
                            .color(Color::WHITE),
                    )
                    .padding([9, 16])
                    .style(brand_button)
                    .on_press(ContainersPageMsg::ApplyUpdate(cid)),
                ],
            ]
            .spacing(14);

            if let Some(st) = &det.update_status {
                form = form.push(text(st).size(12).color(GREEN_400));
            }

            container(form).width(Length::Fill).padding(18).style(dark_card).into()
        }
    };

    col.push(body).width(Length::Fill).height(Length::Fill).into()
}

fn subtab_btn<'a>(
    label: &'static str,
    target: ContainerSubTab,
    current: ContainerSubTab,
    cid: String,
) -> Element<'a, ContainersPageMsg> {
    let active = target == current;
    button(
        text(label)
            .size(12)
            .font(FONT_UI_SEMIBOLD)
            .color(if active { Color::WHITE } else { TEXT_SECONDARY }),
    )
    .padding([7, 14])
    .style(move |_, _| {
        if active {
            brand_button(&crate::theme::glyph_theme(), button::Status::Active)
        } else {
            secondary_button(&crate::theme::glyph_theme(), button::Status::Active)
        }
    })
    .on_press(ContainersPageMsg::SelectSubTab(cid, target))
    .into()
}

fn mini_stat<'a>(label: &'static str, val: &'a str) -> Element<'a, ContainersPageMsg> {
    container(
        column![
            text(label).size(10).color(TEXT_MUTED),
            text(if val.is_empty() { "—" } else { val })
                .size(12)
                .font(FONT_MONO_BOLD)
                .color(TEXT_PRIMARY),
        ]
        .spacing(4),
    )
    .width(Length::FillPortion(1))
    .padding(10)
    .style(dark_card)
    .into()
}
