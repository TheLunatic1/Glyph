use crate::theme::{
    brand_button, danger_ghost_button, dark_card, dark_input, dark_pick_list, dark_scrollable,
    glass_panel, pill_badge, rgba, secondary_button, BLUE_400, BRAND_300, BRAND_400, BRAND_500,
    DARK_700, DARK_800, DARK_900, FONT_MONO, FONT_MONO_BOLD, GREEN_400, GREEN_500, RED_400,
    TEXT_LIGHT, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY, YELLOW_400,
};
use crate::ui::icons;
use glyph_core::models::SavedTunnelRule;
use glyph_core::tunnels::{TunnelDirection, TunnelInfo};
use iced::widget::{
    button, checkbox, column, container, pick_list, row, scrollable, space, text, text_input,
};
use iced::{alignment, Background, Border, Color, Element, Length};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiTunnelDirection {
    Local,
    Remote,
    Dynamic,
}

#[derive(Debug, Clone)]
pub struct TunnelsPageState {
    pub direction: UiTunnelDirection,
    pub bind_port: String,
    pub target_host: String,
    pub target_port: String,
    pub protocol: String,
    pub save_as_profile: bool,
    pub profile_name: String,
    pub profile_auto_start: bool,
    pub tunnels: Vec<TunnelInfo>,
    pub schemes: HashMap<String, bool>, // true = https, false = http
    pub busy: bool,
    pub error: Option<String>,
}

impl Default for TunnelsPageState {
    fn default() -> Self {
        Self {
            direction: UiTunnelDirection::Local,
            bind_port: String::new(),
            target_host: "127.0.0.1".to_string(),
            target_port: String::new(),
            protocol: "TCP".to_string(),
            save_as_profile: false,
            profile_name: String::new(),
            profile_auto_start: false,
            tunnels: Vec::new(),
            schemes: HashMap::new(),
            busy: false,
            error: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum TunnelsPageMsg {
    SelectDirection(UiTunnelDirection),
    BindPortChanged(String),
    TargetHostChanged(String),
    TargetPortChanged(String),
    ProtocolSelected(String),
    ToggleSaveProfile(bool),
    ProfileNameChanged(String),
    ToggleProfileAutoStart(bool),
    BindSubmit,
    StartSavedProfile(String),
    ToggleSavedAutoStart(String),
    DeleteSavedProfile(String),
    ToggleScheme(String),
    OpenUrl(String),
    StopTunnel(String),
    Refresh,
}

pub fn view<'a>(
    st: &'a TunnelsPageState,
    saved_tunnels: &'a [SavedTunnelRule],
) -> Element<'a, TunnelsPageMsg> {
    let header = row![
        container(icons::network(20.0, BRAND_400))
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
            text("Port Forwarding")
                .size(20)
                .font(FONT_MONO_BOLD)
                .color(TEXT_PRIMARY),
            text("Encrypted Local (-L), Remote (-R), Dynamic SOCKS5 (-D), and Saved Profiles")
                .size(12)
                .color(TEXT_SECONDARY),
        ]
        .spacing(2),
        space().width(Length::Fill),
        button(
            row![
                icons::refresh_cw(14.0, TEXT_LIGHT),
                text("Refresh").size(12).color(TEXT_LIGHT),
            ]
            .spacing(6)
            .align_y(alignment::Vertical::Center),
        )
        .padding([8, 12])
        .style(secondary_button)
        .on_press(TunnelsPageMsg::Refresh),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center);

    let dir_pill = |label: &'static str, dir: UiTunnelDirection| {
        let active = st.direction == dir;
        button(
            text(label)
                .size(12)
                .font(FONT_MONO_BOLD)
                .color(if active { Color::WHITE } else { TEXT_SECONDARY }),
        )
        .padding([6, 12])
        .style(move |theme, status| {
            if active {
                brand_button(theme, status)
            } else {
                secondary_button(theme, status)
            }
        })
        .on_press(TunnelsPageMsg::SelectDirection(dir))
    };

    let dir_pills = row![
        dir_pill("Local Forward (-L)", UiTunnelDirection::Local),
        dir_pill("Remote Forward (-R)", UiTunnelDirection::Remote),
        dir_pill("Dynamic SOCKS5 (-D)", UiTunnelDirection::Dynamic),
    ]
    .spacing(8);

    let form_row: Element<'a, TunnelsPageMsg> = if st.direction == UiTunnelDirection::Dynamic {
        let port_hint = if st.bind_port.trim().is_empty() {
            "1080"
        } else {
            st.bind_port.trim()
        };
        let mut r = row![
            text_input("Local SOCKS5 Port (e.g. 1080)", &st.bind_port)
                .on_input(TunnelsPageMsg::BindPortChanged)
                .on_submit(TunnelsPageMsg::BindSubmit)
                .padding([9, 10])
                .size(13)
                .font(FONT_MONO)
                .width(Length::Fixed(220.0))
                .style(dark_input),
            text(format!("socks5h://127.0.0.1:{port_hint}"))
                .size(12)
                .font(FONT_MONO)
                .color(GREEN_400)
                .width(Length::Fill),
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center);

        let bind_btn = button(
            row![
                icons::plug(14.0, Color::WHITE),
                text(if st.busy { "Binding..." } else { "Bind SOCKS5 (-D)" })
                    .size(12)
                    .font(FONT_MONO_BOLD)
                    .color(Color::WHITE),
            ]
            .spacing(6)
            .align_y(alignment::Vertical::Center),
        )
        .padding([9, 14])
        .style(brand_button)
        .on_press_maybe((!st.busy).then_some(TunnelsPageMsg::BindSubmit));
        r = r.push(bind_btn);
        r.into()
    } else {
        let is_local = st.direction == UiTunnelDirection::Local;
        let bind_placeholder = if is_local {
            "Local Port (e.g. 8080, 0=auto)"
        } else {
            "Remote Listen Port (e.g. 9000)"
        };
        let host_placeholder = if is_local {
            "Remote Host (127.0.0.1)"
        } else {
            "Local Host (127.0.0.1)"
        };
        let target_placeholder = if is_local {
            "Remote Port (e.g. 80)"
        } else {
            "Local Port (e.g. 3000)"
        };
        let protocols = vec!["TCP".to_string(), "UDP".to_string()];

        let bind_btn = button(
            row![
                icons::plug(14.0, Color::WHITE),
                text(if st.busy { "Binding..." } else { "Bind Tunnel" })
                    .size(12)
                    .font(FONT_MONO_BOLD)
                    .color(Color::WHITE),
            ]
            .spacing(6)
            .align_y(alignment::Vertical::Center),
        )
        .padding([9, 14])
        .style(brand_button)
        .on_press_maybe((!st.busy).then_some(TunnelsPageMsg::BindSubmit));

        row![
            text_input(bind_placeholder, &st.bind_port)
                .on_input(TunnelsPageMsg::BindPortChanged)
                .padding([9, 10])
                .size(13)
                .font(FONT_MONO)
                .width(Length::FillPortion(2))
                .style(dark_input),
            text("->").size(13).font(FONT_MONO_BOLD).color(TEXT_MUTED),
            text_input(host_placeholder, &st.target_host)
                .on_input(TunnelsPageMsg::TargetHostChanged)
                .padding([9, 10])
                .size(13)
                .font(FONT_MONO)
                .width(Length::FillPortion(2))
                .style(dark_input),
            text(":").size(13).font(FONT_MONO_BOLD).color(TEXT_MUTED),
            text_input(target_placeholder, &st.target_port)
                .on_input(TunnelsPageMsg::TargetPortChanged)
                .on_submit(TunnelsPageMsg::BindSubmit)
                .padding([9, 10])
                .size(13)
                .font(FONT_MONO)
                .width(Length::FillPortion(2))
                .style(dark_input),
            pick_list(
                protocols,
                Some(st.protocol.clone()),
                TunnelsPageMsg::ProtocolSelected
            )
            .padding([8, 12])
            .text_size(12)
            .width(Length::Fixed(86.0))
            .style(dark_pick_list),
            bind_btn,
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center)
        .into()
    };

    let mut save_row = row![
        checkbox(st.save_as_profile).on_toggle(TunnelsPageMsg::ToggleSaveProfile),
        text("Save rule to server profile")
            .size(12)
            .color(TEXT_SECONDARY),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center);

    if st.save_as_profile {
        save_row = save_row
            .push(
                text_input("Profile Name (optional)", &st.profile_name)
                    .on_input(TunnelsPageMsg::ProfileNameChanged)
                    .padding([6, 10])
                    .size(12)
                    .width(Length::Fixed(190.0))
                    .style(dark_input),
            )
            .push(checkbox(st.profile_auto_start).on_toggle(TunnelsPageMsg::ToggleProfileAutoStart))
            .push(text("Auto-start on connect").size(12).color(TEXT_LIGHT));
    }

    let form_card = container(column![dir_pills, form_row, save_row].spacing(10))
        .width(Length::Fill)
        .padding(16)
        .style(glass_panel);

    let mut body = column![header, form_card].spacing(16).padding(24);

    if let Some(err) = &st.error {
        body = body.push(
            container(text(err).size(12).color(RED_400))
                .width(Length::Fill)
                .padding([8, 12])
                .style(dark_card),
        );
    }

    let table_header = row![
        text("Local / Bind Endpoint")
            .size(11)
            .font(FONT_MONO_BOLD)
            .color(TEXT_MUTED)
            .width(Length::FillPortion(3)),
        text("Destination")
            .size(11)
            .font(FONT_MONO_BOLD)
            .color(TEXT_MUTED)
            .width(Length::FillPortion(3)),
        text("Type")
            .size(11)
            .font(FONT_MONO_BOLD)
            .color(TEXT_MUTED)
            .width(Length::Fixed(100.0)),
        text("Status")
            .size(11)
            .font(FONT_MONO_BOLD)
            .color(TEXT_MUTED)
            .width(Length::Fixed(90.0)),
        text("Action")
            .size(11)
            .font(FONT_MONO_BOLD)
            .color(TEXT_MUTED)
            .width(Length::Fixed(64.0)),
    ]
    .padding([8, 12]);

    let mut rows_col = column![table_header].spacing(6);

    if st.tunnels.is_empty() {
        rows_col = rows_col.push(
            container(
                column![
                    icons::network(34.0, TEXT_MUTED),
                    text("No active SSH tunnels on this session.")
                        .size(13)
                        .color(TEXT_SECONDARY),
                ]
                .spacing(8)
                .align_x(alignment::Horizontal::Center),
            )
            .width(Length::Fill)
            .padding(32)
            .align_x(alignment::Horizontal::Center),
        );
    } else {
        for tun in &st.tunnels {
            let tid = tun.id.clone();
            let is_https = *st.schemes.get(&tid).unwrap_or(&false);
            let scheme = if is_https { "https" } else { "http" };

            let endpoint_cell: Element<'a, TunnelsPageMsg> = match tun.direction {
                TunnelDirection::Local => {
                    let url = format!("{scheme}://127.0.0.1:{}", tun.local_port);
                    row![
                        button(
                            text(scheme)
                                .size(10)
                                .font(FONT_MONO_BOLD)
                                .color(BRAND_300),
                        )
                        .padding([3, 7])
                        .style(secondary_button)
                        .on_press(TunnelsPageMsg::ToggleScheme(tid.clone())),
                        button(
                            row![
                                text(format!("localhost:{}", tun.local_port))
                                    .size(12)
                                    .font(FONT_MONO_BOLD)
                                    .color(BLUE_400),
                                icons::external_link(12.0, BLUE_400),
                            ]
                            .spacing(5)
                            .align_y(alignment::Vertical::Center),
                        )
                        .padding([4, 6])
                        .style(|_, _| button::Style {
                            background: None,
                            snap: true,
                            ..Default::default()
                        })
                        .on_press(TunnelsPageMsg::OpenUrl(url)),
                    ]
                    .spacing(6)
                    .align_y(alignment::Vertical::Center)
                    .width(Length::FillPortion(3))
                    .into()
                }
                TunnelDirection::Remote => text(format!("remote:{}", tun.remote_port))
                    .size(12)
                    .font(FONT_MONO_BOLD)
                    .color(BRAND_300)
                    .width(Length::FillPortion(3))
                    .into(),
                TunnelDirection::Dynamic => text(format!("socks5://127.0.0.1:{}", tun.local_port))
                    .size(12)
                    .font(FONT_MONO_BOLD)
                    .color(GREEN_400)
                    .width(Length::FillPortion(3))
                    .into(),
            };

            let dest_label = match tun.direction {
                TunnelDirection::Local => format!("{}:{}", tun.remote_host, tun.remote_port),
                TunnelDirection::Remote => format!("{}:{}", tun.remote_host, tun.local_port),
                TunnelDirection::Dynamic => "Dynamic Proxy".to_string(),
            };
            let dir_badge = match tun.direction {
                TunnelDirection::Local => format!("-L {}", tun.protocol),
                TunnelDirection::Remote => format!("-R {}", tun.protocol),
                TunnelDirection::Dynamic => "-D SOCKS5".to_string(),
            };

            let tun_row = container(
                row![
                    endpoint_cell,
                    text(dest_label)
                        .size(12)
                        .font(FONT_MONO)
                        .color(TEXT_LIGHT)
                        .width(Length::FillPortion(3)),
                    container(
                        container(
                            text(dir_badge)
                                .size(11)
                                .font(FONT_MONO_BOLD)
                                .color(BRAND_300),
                        )
                        .padding([3, 8])
                        .style(pill_badge(
                            rgba(BRAND_500, 0.14),
                            rgba(BRAND_500, 0.3),
                            BRAND_300,
                        )),
                    )
                    .width(Length::Fixed(100.0)),
                    container(
                        container(
                            text("Active")
                                .size(11)
                                .font(FONT_MONO_BOLD)
                                .color(GREEN_400),
                        )
                        .padding([3, 8])
                        .style(pill_badge(
                            rgba(GREEN_500, 0.14),
                            rgba(GREEN_500, 0.35),
                            GREEN_400,
                        )),
                    )
                    .width(Length::Fixed(90.0)),
                    container(
                        button(icons::trash(14.0, RED_400))
                            .padding([6, 10])
                            .style(danger_ghost_button)
                            .on_press(TunnelsPageMsg::StopTunnel(tid)),
                    )
                    .width(Length::Fixed(64.0)),
                ]
                .align_y(alignment::Vertical::Center),
            )
            .width(Length::Fill)
            .padding([10, 12])
            .style(|_| container::Style {
                background: Some(Background::Color(DARK_900)),
                border: Border {
                    color: DARK_700,
                    width: 1.0,
                    radius: 8.0.into(),
                },
                snap: true,
                ..Default::default()
            });

            rows_col = rows_col.push(tun_row);
        }
    }

    let table_card = container(
        column![
            row![
                text("Active Tunnels")
                    .size(14)
                    .font(FONT_MONO_BOLD)
                    .color(TEXT_PRIMARY),
                space().width(Length::Fill),
                text(format!("{} bound", st.tunnels.len()))
                    .size(11)
                    .font(FONT_MONO)
                    .color(TEXT_MUTED),
            ]
            .align_y(alignment::Vertical::Center),
            scrollable(rows_col)
                .height(Length::Fill)
                .style(dark_scrollable),
        ]
        .spacing(10),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(16)
    .style(|_| container::Style {
        background: Some(Background::Color(DARK_800)),
        border: Border {
            color: DARK_700,
            width: 1.0,
            radius: 12.0.into(),
        },
        snap: true,
        ..Default::default()
    });

    body = body.push(
        container(table_card)
            .width(Length::Fill)
            .height(Length::FillPortion(3)),
    );

    let mut saved_col = column![].spacing(6);
    if saved_tunnels.is_empty() {
        saved_col = saved_col.push(
            text("No saved tunnel profiles yet. Check \"Save rule to server profile\" above when binding a tunnel.")
                .size(12)
                .color(TEXT_MUTED),
        );
    } else {
        for rule in saved_tunnels {
            let desc = match rule.direction {
                TunnelDirection::Local => format!(
                    "-L {}:{} -> {}:{} ({})",
                    rule.local_port, rule.protocol, rule.remote_host, rule.remote_port, rule.protocol
                ),
                TunnelDirection::Remote => format!(
                    "-R {} -> {}:{} ({})",
                    rule.remote_port, rule.remote_host, rule.local_port, rule.protocol
                ),
                TunnelDirection::Dynamic => format!("-D SOCKS5 127.0.0.1:{}", rule.local_port),
            };
            let rid = rule.id.clone();
            saved_col = saved_col.push(
                container(
                    row![
                        column![
                            text(&rule.name)
                                .size(13)
                                .font(FONT_MONO_BOLD)
                                .color(TEXT_PRIMARY),
                            text(desc).size(11).font(FONT_MONO).color(TEXT_SECONDARY),
                        ]
                        .spacing(2)
                        .width(Length::Fill),
                        button(
                            text(if rule.auto_start {
                                "Auto-Start: ON"
                            } else {
                                "Auto-Start: OFF"
                            })
                            .size(11)
                            .color(if rule.auto_start {
                                YELLOW_400
                            } else {
                                TEXT_MUTED
                            }),
                        )
                        .padding([5, 9])
                        .style(secondary_button)
                        .on_press(TunnelsPageMsg::ToggleSavedAutoStart(rid.clone())),
                        button(
                            row![
                                icons::play(11.0, Color::WHITE),
                                text("Start").size(11).color(Color::WHITE),
                            ]
                            .spacing(5)
                            .align_y(alignment::Vertical::Center),
                        )
                        .padding([5, 10])
                        .style(brand_button)
                        .on_press(TunnelsPageMsg::StartSavedProfile(rid.clone())),
                        button(icons::trash(13.0, RED_400))
                            .padding([5, 8])
                            .style(danger_ghost_button)
                            .on_press(TunnelsPageMsg::DeleteSavedProfile(rid)),
                    ]
                    .spacing(8)
                    .align_y(alignment::Vertical::Center),
                )
                .width(Length::Fill)
                .padding([8, 12])
                .style(dark_card),
            );
        }
    }

    let saved_card = container(
        column![
            row![
                text("Saved Tunnel Profiles")
                    .size(14)
                    .font(FONT_MONO_BOLD)
                    .color(TEXT_PRIMARY),
                space().width(Length::Fill),
                text(format!("{} profiles", saved_tunnels.len()))
                    .size(11)
                    .font(FONT_MONO)
                    .color(TEXT_MUTED),
            ]
            .align_y(alignment::Vertical::Center),
            scrollable(saved_col)
                .height(Length::Fill)
                .style(dark_scrollable),
        ]
        .spacing(8),
    )
    .width(Length::Fill)
    .height(Length::FillPortion(2))
    .padding(14)
    .style(glass_panel);

    body = body.push(saved_card);
    body.width(Length::Fill).height(Length::Fill).into()
}
